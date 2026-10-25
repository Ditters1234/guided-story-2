import { mount } from 'svelte';
import App from './App.svelte';
import './style.css';
import './themes.css';
import './authoring.css';
import { applyAppearance, defaultPreferences } from './lib/appearance';

applyAppearance(defaultPreferences(), window.matchMedia('(prefers-color-scheme: dark)').matches);
mount(App, { target: document.getElementById('app')! });
