<script lang="ts">
  import Capture from '@glassboard/ui/Capture.svelte';
  import type { Session } from '@glassboard/ui/session';
  import { getCaptureImage, copyCaptureImage } from './lib/capture';

  let { session, onerror }: { session: Session; onerror: (error: unknown) => void } = $props();
</script>

<Capture {session} {onerror} getImage={getCaptureImage} copyImage={copyCaptureImage} />
