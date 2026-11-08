import type {AiContextBinding, TextPromptTask} from './bindings';
import type {World} from './model';
import {defaultContextBinding} from './ai-context';

/** Suggestions only: opening review never opts a source into a tool. */
export function starterContextDraft(world: World): World {
  const draft = structuredClone(world);
  const bindings = draft.aiContext ?? [];
  for (const node of draft.nodes) {
    if (bindings.some(binding => binding.nodeId === node.id)) continue;
    const components = node.components;
    let suggestion: AiContextBinding;
    if (components['core.narration-style'] || components['detail.visual-style']) {
      suggestion = defaultContextBinding(node.id, bindings.length * 10, draft);
    } else {
      const instructions = !!(components['detail.rule'] || components['detail.brief']);
      if (!instructions && !components['detail.time'] && !components['detail.lore']) continue;
      suggestion = {
        nodeId: node.id, enabled: false, role: instructions ? 'instructions' : 'reference',
        tasks: components['detail.brief'] ? ['planning'] : ['passage', 'wording', 'planning'],
        fields: ['description', 'components'], order: bindings.length * 10,
      };
    }
    bindings.push({...suggestion, enabled: false});
  }
  return {...draft, aiContext: bindings};
}

/** Bind only selected sources; node facts, links and definitions stay unchanged. */
export function selectedStarterContext(source: World, bindings: AiContextBinding[]): World {
  const {aiContext: _previous, ...base} = source;
  const selected = bindings.filter(binding => binding.enabled);
  return selected.length ? {...base, aiContext: structuredClone(selected)} : base;
}

export async function reviewStarterContext(
  world: World,
  validate: (world: World) => Promise<unknown>,
  preview: (world: World, task: TextPromptTask) => Promise<unknown>,
  previewImage: (world: World) => Promise<unknown>,
): Promise<void> {
  await validate(world);
  const bindings = world.aiContext ?? [];
  const tasks = [...new Set(bindings.filter(b => b.enabled).flatMap(b => b.tasks))];
  await Promise.all([
    ...tasks.map(task => preview(world, task)),
    ...(bindings.some(b => b.enabled && b.imagePreparation) ? [previewImage(world)] : []),
  ]);
}
