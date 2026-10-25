<script lang="ts">
  // Adapted from FrostyBee's LeftSideBar wrapper, MIT. See THIRD_PARTY_NOTICES.md.
  import type { Snippet } from 'svelte';
  let { children, label }: { children: Snippet; label: string } = $props();
</script>

<aside class="sidebar" aria-label={label}>
  {@render children()}
</aside>
