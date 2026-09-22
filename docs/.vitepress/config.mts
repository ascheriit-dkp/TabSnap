import { defineConfig } from 'vitepress';

export default defineConfig({
  title: 'TabSnap',
  description: 'Move a browser workspace between machines without a backend.',
  base: '/TabSnap/',
  themeConfig: {
    nav: [
      { text: 'Guide', link: '/guide/' },
      { text: 'Chrome', link: '/guide/chrome-extension' },
      { text: 'Edge', link: '/guide/edge-extension' },
      { text: 'Firefox', link: '/guide/firefox-extension' },
      { text: 'Whole-machine', link: '/guide/windows-companion-ui' },
      { text: 'Format', link: '/technical/tabsnap-format' },
      { text: 'Privacy', link: '/privacy' },
      { text: 'Security', link: '/security/' },
      { text: 'Architecture', link: '/architecture/' },
      { text: 'Roadmap', link: '/roadmap' },
    ],
    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'Overview', link: '/guide/' },
          { text: 'Chrome extension', link: '/guide/chrome-extension' },
          { text: 'Microsoft Edge', link: '/guide/edge-extension' },
          { text: 'Firefox', link: '/guide/firefox-extension' },
          { text: 'Windows companion', link: '/guide/windows-companion' },
          { text: 'Whole-machine UI', link: '/guide/windows-companion-ui' },
          { text: 'Store submission', link: '/guide/store-submission' },
          { text: 'Development', link: '/guide/development' },
        ],
      },
      {
        text: 'Technical',
        items: [
          { text: '.tabsnap format', link: '/technical/tabsnap-format' },
          { text: 'Firefox compatibility', link: '/technical/firefox-compatibility' },
          { text: 'Cross-browser compatibility', link: '/technical/cross-browser-compatibility' },
        ],
      },
      {
        text: 'Project',
        items: [
          { text: 'Privacy', link: '/privacy' },
          { text: 'Security', link: '/security/' },
          { text: '1.0 readiness', link: '/security/level4-readiness' },
          { text: 'Architecture', link: '/architecture/' },
          { text: 'Roadmap', link: '/roadmap' },
        ],
      },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/ascheriit-dkp/TabSnap' }],
  },
});
