<script lang="ts">
  import Capture from '@glassboard/ui/Capture.svelte';
  import { native, type Session } from '@glassboard/ui/session';
  import { getCaptureImage, copyCaptureImage, copyCaptureRegion } from './lib/capture';

  let { session, onerror }: { session: Session; onerror: (error: unknown) => void } = $props();
</script>

<Capture {session} {onerror} getImage={getCaptureImage} copyImage={copyCaptureImage} copyRegion={native ? copyCaptureRegion : undefined} />
