import { mount } from 'svelte';
import './styles/global.css';
import Root from './Root.svelte';

const target = document.getElementById('app');
if (target) {
  mount(Root, { target });
}
