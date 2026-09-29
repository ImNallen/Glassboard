import { mount } from 'svelte';
import App from './App.svelte';
import '@glassboard/ui/toolbar.css';
import './style.css';
mount(App, { target: document.getElementById('app')! });
