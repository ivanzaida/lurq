# Розкладка lurq проти taffy: план оптимізацій

Гілка: `claude/layout-perf`. Вимірювання: `cargo bench -p lurq --bench vs_taffy`
(час) і `cargo bench -p lurq --bench vs_taffy_allocs` (алокації). Обидва
рушії будують однакові дерева, і перевірка збігу позицій листків проходить на всіх сценаріях.

Правило для кожного кроку: окремий коміт; повний набір тестів
(`cargo test -p lurq --features markdown,canvas,query,tokio,svg,image,screenshot,form,router,i18n --no-fail-fast`)
з тим самим результатом, що й до змін; регресійний тест на виправлену
поведінку; заміри до/після в описі коміту.

## Що виміряно до змін

| режим | lurq проти taffy |
|---|---|
| повний перерахунок | 1.6–2.4x повільніше на плоских списках, 0.4–0.7x (швидше) на вкладених деревах |
| нічого не змінилось (noop) | 6 µs – 4.5 ms проти ~30 ns; росте з розміром дерева |
| зміна ширини кореня | до 155x повільніше (661 µs проти 4 µs на 5 461 вузлі) |
| побудова дерева | 2–3x повільніше, ~4 алокації на вузол |
| стек | ~10 KB на рівень вкладеності проти ~2.5 KB; на 1 MB стеку Windows переповнюється на ~100 рівнях |

## Кроки

### 1. Копіювання кешованих піддерев і повторний обхід (зроблено, не закомічено)
- `LayoutResult::holds` фіксує, чи є в піддереві скрол або вивід
  тексту; `prepare_layout_result_tree` заходить і копіює лише туди.
- `position_flex_line` переміщує результати дітей, а не клонує.
- Тести: `serving_an_unchanged_tree_shares_its_cached_subtrees`,
  `holds_record_the_scroll_containers_and_texts_below`,
  `nested_scroll_container_follows_its_scroll_through_cached_and_fresh_layouts`.
- Алокації: noop 1 на все дерево (було 1 на вузол); глибокий ланцюжок
  ~8 на вузол незалежно від глибини (було до 102).

### 2. Стек: ~10 KB на рівень → мета ≤ 4 KB
Найбільший ризик для реальних застосунків: переповнення аварійно завершує процес.
- Знайти великі кадри у рекурсії `layout_node` → `layout_node_box` →
  `layout_flat_box` → `layout_node_content` → `layout_flex` →
  `layout_child_node`.
- Підозрювані: `Style`, який `state_style()` будує на стеку; гілки
  для тексту/полів вводу/select у `layout_leaf`, чиї локальні змінні роздувають
  кадр навіть для простого прямокутника; `LayoutResult` за значенням.
- Підхід: рідкісні гілки винести в `#[inline(never)]`-функції, великі
  локальні змінні тримати в купі або зробити коротшими.
- Уже виміряно (release, коміт `1b6dbbd`):
  - **Маркер адреси стеку в `layout_node`:** сама рекурсія розкладки бере
    ~10 KB на рівень сценарію `deep_chain` (10 рівнів: 108 912 B, 80 рівнів:
    801 072 B). Тобто це не `set_root` і не drop.
  - **Кадри з прологів функцій у release-`rlib`**
    (`llvm-objdump -d` по об'єктах з `liblurq-*.rlib`):
    `layout_node_uncached_with_child_overrides` 1104 B (сюди вбудовані
    `layout_node_box`/`layout_flat_box`), `layout_flex` 976,
    `layout_node_from_cache` 576, `effective_frame` 560, `state_style` 464,
    `effective_padding` 448, `state_flex` 432, `layout_child_node` 288,
    `layout_node` 160, `layout_node_content` 72.
  - **Розбіжність:** очевидний цикл
    (`layout_node` → `layout_node_uncached_with_child_overrides` →
    `layout_node_content` → `layout_flex` → `layout_child_node`) дає лише
    ≈2.6 KB. Решта ~7 KB на рівень іде на ще не знайдені кадри.
  - **Наступний крок:** порахувати вузли на рівень (`Column::with(..).padding(..)`
    може давати обгортку) і пройти реальний ланцюжок викликів (наприклад,
    маркерами в кожній функції циклу).
- Перевірка: стек-проба в `vs_taffy` (`VS_TAFFY_PROBE`) і
  `scripts/stack-depth-probe.py` з наявним тестом
  `runtime::stack_depth::...`.

### 3. Сталі витрати на вузол у повному перерахунку (~1.4–2.4x)
- `state_style()` збирає цілий `Style` при кожному виклику
  `effective_frame`, `effective_padding`, `state_flex`, `state_frame`,
  кілька разів на вузол. Швидкий шлях: немає активного стану
  (hover/focus/active/drag) → нічого не зливати.
- `LayoutCache`: `RefCell<Vec<CachedLayout>>` із `insert(0, ..)` і
  клоном результату на кожне збереження → масив на 2 записи без зсувів.
- Вимір: `forced` на `flat_column/10000` і `grow_rows/1000`.

### 4. Незмінене дерево досі O(n)
Після кроку 1 без алокацій, але все одно обходить усе дерево:
- `mark_layout_dirty` обходить усе дерево на кожному `compute`;
- `cached_result_matches_node_tree` обходить усе піддерево при влученні в кеш кореня;
- `update_layout` обходить дерево ще чотирма рекурсивними перевірками
  (`has_dirty_element_ref_recursive`, `has_pending_layout_dirty_recursive`,
  `has_runtime_layout_state_recursive`, `has_timeline_target_recursive`).
- Підхід: позначати бруд у момент зміни. Прапорець «піддерево могло
  змінитись» ставиться при reconcile/preserve, мутаціях через handle і
  в анімаціях; перевірки `cached_result_matches_node_tree` лишити тільки для
  вузлів, чий кеш перенесено під час останньої перебудови.
- Найризиковіший крок: ці обходи закривають баги, які раніше траплялися в продакшні
  (коментарі в коді). Робити після 2–3, з окремими тестами на кожен
  такий сценарій.

### 5. Побудова дерева (2–3x, ~4 алокації на вузол)
- Побудова глибокого ланцюжка надлінійна (64 рівні: 8x проти taffy, 16: 3x):
  знайти, що в `with_children`/`set_root` залежить від глибини.
- Розмір `Node` і алокації на вузол.

### 6. Наявні бенчмарки `layout` і `render_list` не міряють розкладку
`Tree::rebuild()` без кореневого компонента одразу повертається, тож вони міряють
лише `set_root`. Перевести на `pass_headless`/`bench_layout_only`.
