<script lang="ts">
  import { t } from '@glassboard/ui/i18n';
  import Listbox from './Listbox.svelte';
  import { LANGUAGES } from './lib/languages';
  /** `language` is the saved request. A tag this build doesn't list, saved by a newer one, reads as the system default. */
  let { language, onselect }: { language: string; onselect: (language: string) => void } = $props();
  type Option = (typeof OPTIONS)[number];
  const SYSTEM = { tag: 'system', name: '' };
  const OPTIONS = [SYSTEM, ...LANGUAGES];
  const label = (option: Option) => option === SYSTEM ? t('System default') : option.name;
  let selected = $derived(OPTIONS.find(option => option.tag === language) ?? SYSTEM);
</script>

{#snippet name(option: Option)}
  <span class="name" lang={option === SYSTEM ? undefined : option.tag}>{label(option)}</span>
{/snippet}

<Listbox options={OPTIONS} {selected} id="language-list" label={t('Language: {language}', { language: label(selected) })} listLabel={t('Languages')} dividerAfter={0} onselect={option => onselect(option.tag)}>
  {#snippet trigger()}{@render name(selected)}{/snippet}
  {#snippet option(choice: Option)}{@render name(choice)}{/snippet}
</Listbox>

<style>
  .name { flex: 1; min-width: 0; font-size: 12.5px; font-weight: 500; color: var(--gb-text); }
</style>
