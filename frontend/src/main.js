import { mount } from 'svelte';
import './styles/global.css';
import Root from './Root.svelte';

// Optional Umami statistics: on when the server has UMAMI_URL and UMAMI_WEBSITE_ID
fetch('/api/analytics')
  .then((r) => (r.ok ? r.json() : null))
  .then((u) => {
    if (!u?.script || !u?.website_id) return;
    const s = document.createElement('script');
    s.defer = true;
    s.src = u.script;
    s.dataset.websiteId = u.website_id;
    document.head.appendChild(s);
  })
  .catch(() => {});

const target = document.getElementById('app');
if (target) {
  mount(Root, { target });
}
