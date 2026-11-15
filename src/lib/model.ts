// Contracts come from Rust; presets are shared data, not fixed entity classes.
export type {
  World_Serialize as World, WorldNode as WorldEntity,
  ProjectView_Serialize as ProjectView, Workspace_Serialize as Workspace,
  EditorDraft_Serialize as EditorDraft, NodeTypeDefinition,
} from './bindings';
import type { World, WorldEntity } from './model';
import presets from '../../schema/world-presets.json';
import {defaultProperties} from './node-fields';
import {withBriefFields} from './writing-inputs';
export const emptyWorld = (): World => withBriefFields(structuredClone(presets) as World);
export function availableNodeTypes(world:World){return [...world.nodeTypes,...presets.nodeTypes.filter(p=>!world.nodeTypes.some(t=>t.id===p.id))];}
export function withBuiltinNodeType(world:World,typeId:string):World {
  const saved=world.nodeTypes.find(t=>t.id===typeId);
  if(saved)return saved.components.includes('detail.brief')?withBriefFields(world):world;
  const preset=presets.nodeTypes.find(t=>t.id===typeId);if(!preset)return world;
  const capabilities=presets.componentTypes.filter(d=>preset.components.includes(d.id)&&!world.componentTypes.some(c=>c.id===d.id));
  const relationships=presets.relationTypes.filter(d=>d.id==='core.holds-claim'&&preset.components.includes('core.claim')&&!world.relationTypes.some(r=>r.id===d.id));
  const next={...world,nodeTypes:[...world.nodeTypes,structuredClone(preset)],componentTypes:[...world.componentTypes,...structuredClone(capabilities)],relationTypes:[...world.relationTypes,...structuredClone(relationships)]} as World;
  return preset.components.includes('detail.brief')?withBriefFields(next):next;
}
export function makeNode(world: World, typeId: string, name: string, id: string = crypto.randomUUID()): WorldEntity {
  const preset = world.nodeTypes.find((d) => d.id === typeId);
  return { id, typeId, name, description: '',
    components: Object.fromEntries((preset?.components ?? []).map((id) => {
      const definition = world.componentTypes.find((d) => d.id === id);
      return [id, definition?defaultProperties(definition):{}];
    })) };
}
