<script lang="ts">
  import { shortcutLabel } from '@glassboard/ui/shortcuts';
  import type { Action, Session } from '@glassboard/ui/session';
  let { session, run }: { session: Session; run: (name: Action) => void } = $props();
</script>

<main class="preview-background">
  <div class="preview-top"><span class="preview-logo">↗ Glassboard</span><span class="preview-badge">BROWSER PREVIEW</span></div>
  <div class="preview-copy"><span class="eyebrow">A LITTLE CLARITY GOES A LONG WAY</span><h1>Your screen.<br/>Your point.</h1><p>Draw attention to what matters.<br/>Pick a tool below and make your mark.</p><div class="preview-shortcuts"><kbd>{shortcutLabel(session.preferences.shortcut)}</kbd><span>Toggle Glassboard</span><kbd>Shift</kbd><span>Constrain shapes</span></div></div>
  <div class="preview-footer"><span>Arrows. Shapes. A little emphasis.</span><span>Nothing between you and your point.</span></div>
</main>
{#if session.mode === 'hidden'}<div class="preview-actions"><button class="preview-restore" onclick={() => run('toggle')}>Start annotating</button></div>{/if}

<style>
  .preview-background { min-height: 100vh; padding: 36px 48px; background: #f0f0ec; color: #252a30; background-image: radial-gradient(#424b6110 1px, transparent 1px); background-size: 24px 24px; }
  .preview-top { display: flex; justify-content: space-between; align-items: center; }
  .preview-logo { font-size: 17px; font-weight: 600; letter-spacing: -.5px; }
  .preview-badge { font-size: 9px; letter-spacing: 1.4px; color: #7d8589; border: 1px solid #252a301a; border-radius: 20px; padding: 7px 10px; }
  .preview-copy { margin: 12vh 0 0 8vw; }
  .eyebrow { font-size: 9px; font-weight: 600; letter-spacing: 1.8px; color: #80897c; }
  h1 { font-size: clamp(50px, 7vw, 96px); font-weight: 500; letter-spacing: -5px; line-height: 1.04; margin: 24px 0; }
  .preview-copy p { color: #848a89; line-height: 1.8; font-size: 15px; }
  .preview-shortcuts { display: flex; align-items: center; gap: 10px; font-size: 10px; color: #919795; margin-top: 28px; }
  .preview-shortcuts kbd { color: #68716f; background: #e5e6e1; border: 1px solid #d9dcd5; }
  .preview-shortcuts span { margin-right: 12px; }
  .preview-footer { position: absolute; bottom: 110px; left: 48px; right: 48px; display: flex; justify-content: space-between; color: #a2a7a2; font-size: 10px; }
  .preview-actions { position: fixed; z-index: 20; bottom: 25px; left: 50%; transform: translateX(-50%); display: flex; gap: 10px; }
  .preview-restore { padding: 14px 20px; border-radius: 12px; background: var(--gb-surface); color: var(--gb-text); border: 1px solid var(--gb-border); }
</style>
