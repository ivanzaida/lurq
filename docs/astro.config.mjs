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
                        'animation-transforms',
                        'styling-events',
                        'forms',
                        'routing',
                        'futures-timers',
                        'i18n',
                        'modals',
                        'app-runtime',
                        'devtools',
                        'resources-media',
                        'testing',
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
