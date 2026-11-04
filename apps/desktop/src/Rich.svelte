<script lang="ts" generics="K extends RichKey">
  import type { Snippet } from 'svelte';
  import { parts, type Args, type RichKey, type RichTag } from '@glassboard/ui/i18n';
  /** A sentence with markup. The caller's snippets own each tag's element, so the caller's scoped CSS applies to it. */
  let { message, args, ...tags }: { message: K; args: Args<K> } & Record<RichTag<K>, Snippet<[string]>> = $props();
</script>

{#each parts(message, args) as part}{#if part.tag}{@render (tags as unknown as Record<string, Snippet<[string]>>)[part.tag](part.text)}{:else}{part.text}{/if}{/each}
