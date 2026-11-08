import {describe, expect, it, vi} from 'vitest';
import templates from '../../schema/location-templates.json';
import type {PropertyValue} from './bindings';
import {emptyWorld, makeNode} from './model';
import {reviewStarterContext, selectedStarterContext, starterContextDraft} from './starter-context';

function fixture(index = 0) {
  const world = emptyWorld();
  world.nodes = templates[index].nodes.map(seed => {
    const node = {...makeNode(world, seed.typeId, seed.name), description: seed.description, visualDescription: seed.visualDescription};
    for (const [capability, values] of Object.entries(seed.components ?? {})) {
      node.components[capability] = {...node.components[capability], ...values} as Record<string, PropertyValue>;
    }
    return node;
  });
  return world;
}

describe('reviewed starter context', () => {
  it('offers every bundled starter six capability-based choices, all initially off', () => {
    for (let index = 0; index < templates.length; index++) {
      const world = fixture(index), original = structuredClone(world), draft = starterContextDraft(world);
      expect(draft.aiContext).toHaveLength(6);
      expect(draft.aiContext?.every(binding => !binding.enabled && !binding.fields.includes('visualDescription'))).toBe(true);
      const bindingFor = (capability: string) => draft.aiContext!.find(b => draft.nodes.find(n => n.id === b.nodeId)?.components[capability]);
      expect(bindingFor('core.narration-style')).toMatchObject({role: 'instructions', tasks: ['passage', 'wording']});
      expect(bindingFor('detail.visual-style')).toMatchObject({role: 'instructions', tasks: ['imagePrompt'], imagePreparation: true});
      expect(bindingFor('detail.time')).toMatchObject({role: 'reference', tasks: ['passage', 'wording', 'planning']});
      expect(bindingFor('detail.rule')).toMatchObject({role: 'instructions', tasks: ['passage', 'wording', 'planning']});
      expect(bindingFor('detail.lore')).toMatchObject({role: 'reference', tasks: ['passage', 'wording', 'planning']});
      expect(bindingFor('detail.brief')).toMatchObject({role: 'instructions', tasks: ['planning']});
      expect(bindingFor('detail.environment')).toBeUndefined();
      expect(draft.nodes).toEqual(original.nodes);
      expect(draft.edges).toEqual(original.edges);
      expect(world).toEqual(original);
      expect(selectedStarterContext(world, draft.aiContext!)).toEqual(original);
    }
  });

  it('keeps only explicit selections and preserves source facts, schemas and configured choices on rereview', () => {
    const source = fixture(), original = structuredClone(source), draft = starterContextDraft(source);
    // Preset names do not determine inclusion capability behavior.
    draft.nodes.forEach(node => { node.typeId = 'custom.renamed'; });
    const renamed = starterContextDraft({...draft, aiContext: []});
    expect(renamed.aiContext?.map(b => b.nodeId)).toEqual(draft.aiContext?.map(b => b.nodeId));
    const selected = {...draft.aiContext![0], enabled: true, role: 'instructions' as const, tasks: ['planning' as const], fields: ['components' as const], order: -7};
    const result = selectedStarterContext(source, [selected, ...draft.aiContext!.slice(1)]);
    expect(result.nodes).toEqual(original.nodes);
    expect(result.edges).toEqual(original.edges);
    expect(result.nodeTypes).toEqual(original.nodeTypes);
    expect(result.componentTypes).toEqual(original.componentTypes);
    expect(result.relationTypes).toEqual(original.relationTypes);
    expect(result.aiContext).toEqual([selected]);
    expect(starterContextDraft(result).aiContext?.filter(b => b.nodeId === selected.nodeId)).toEqual([selected]);
    selected.tasks.push('planning');
    expect(result.aiContext![0].tasks).toEqual(['planning']);
    expect(source).toEqual(original);
  });

  it('validates all selected native task previews once, with image guidance separately and no disabled tasks', async () => {
    const source = fixture(), draft = starterContextDraft(source);
    const enabled = draft.aiContext!.map(b => ({...b, enabled: true}));
    enabled.push({...enabled[0], nodeId: 'disabled-source', enabled: false, tasks: ['effects']});
    const selected = selectedStarterContext(source, enabled);
    const events: string[] = [];
    await reviewStarterContext(selected, async () => { events.push('validate'); }, async (_, task) => { events.push(task); }, async () => { events.push('imagePreparation'); });
    expect(events[0]).toBe('validate');
    expect(events.slice(1).sort()).toEqual(['imagePreparation', 'imagePrompt', 'passage', 'planning', 'wording']);
  });

  it('retains reviewable choices on validation or preview failure instead of creating a world', async () => {
    const draft = starterContextDraft(fixture());
    draft.aiContext![0].enabled = true;
    const selected = selectedStarterContext(draft, draft.aiContext!), original = structuredClone(selected);
    const preview = vi.fn(async () => {}), image = vi.fn(async () => {});
    await expect(reviewStarterContext(selected, async () => { throw Error('Invalid selected source'); }, preview, image)).rejects.toThrow('Invalid selected source');
    expect(preview).not.toHaveBeenCalled();
    expect(image).not.toHaveBeenCalled();
    await expect(reviewStarterContext(selected, async () => {}, async () => { throw Error('Required context is empty'); }, image)).rejects.toThrow('Required context is empty');
    expect(selected).toEqual(original);
    expect(draft.aiContext![0].enabled).toBe(true);
  });
});
