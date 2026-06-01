import {defineConfig} from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
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
                        'styling-events',
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
