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
      { text: 'Format', link: '/technical/tabsnap-format' },
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
          { text: 'Development', link: '/guide/development' },
        ],
      },
      {
        text: 'Technical',
        items: [
          { text: '.tabsnap format', link: '/technical/tabsnap-format' },
          { text: 'Firefox compatibility', link: '/technical/firefox-compatibility' },
        ],
      },
      {
        text: 'Project',
        items: [
          { text: 'Security', link: '/security/' },
          { text: 'Architecture', link: '/architecture/' },
          { text: 'Roadmap', link: '/roadmap' },
        ],
      },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/ascheriit-dkp/TabSnap' }],
  },
});
