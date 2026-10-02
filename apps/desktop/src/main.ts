import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import { useSession } from '@glassboard/ui/session';
import { nativeSession } from './lib/native-session';
import App from './App.svelte';
import '@glassboard/ui/toolbar.css';
import './style.css';
if (isTauri()) useSession(nativeSession, { native: true });
mount(App, { target: document.getElementById('app')! });
