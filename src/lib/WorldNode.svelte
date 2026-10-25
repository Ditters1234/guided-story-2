<script lang="ts">
  import { Handle, Position, type NodeProps } from '@xyflow/svelte';
  import type { WorldNode } from './graph';
  let { data, selected }: NodeProps<WorldNode> = $props();
  const symbols = { place: '⌂', character: '◉', item: '◇', thread: '↝', custom: '✧' };
</script>

<div class:selected class="world-node {data.kind}">
  <Handle type="source" position={Position.Left} />
  <div class="node-main">
    <span class="node-symbol" aria-hidden="true">{data.kind === 'character' ? data.name.split(' ').map((s) => s[0]).slice(0, 2).join('') : symbols[data.kind]}</span>
    <div><span class="node-kind">{data.detail}</span><strong>{data.name || 'Unnamed record'}</strong></div>
  </div>
  {#if data.count}
    <button class="node-expand nodrag" aria-label={`${data.expanded ? 'Collapse' : 'Expand'} ${data.name}`} onclick={(event) => { event.stopPropagation(); data.onToggle(data.id); }}>
      <span>{data.count} connected</span><span>{data.expanded ? '−' : '+'}</span>
    </button>
  {/if}
  <Handle type="target" position={Position.Right} />
</div>
