import {defineConfig} from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
    site: 'https://ivanzaida.github.io',
    base: '/lurq',
    integrations: [
        starlight({
            title: 'lurq',
            description: 'Documentation for the lurq Rust UI toolkit.',
            sidebar: [
                {
                    label: 'Guides',
                    items: [
                        'getting-started',
                        'mental-model',
                        'components',
                        'reactivity',
                        'layout',
                        'theme',
                        'design-system',
                        'animation-transforms',
                        'styling-events',
                        'blur',
                        'glass',
                        'focus-navigation',
                        'forms',
                        'routing',
                        'futures-timers',
                        'queries',
                        'persistent-storage',
                        'i18n',
                        'dates',
                        'modals',
                        'app-runtime',
                        'window-lifecycle-menus',
                        'ios',
                        'web',
                        'devtools',
                        'mcp',
                        'resources-media',
                        'canvas',
                        'gpu-viewport',
                        'testing',
                        'text-pipeline-optimization',
                    ],
                },
                {
                    label: 'Reference',
                    items: [
                        'ctx',
                        'dsl',
                        'retained_nodes',
                    ],
                },
            ],
        }),
    ],
});
