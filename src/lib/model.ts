// Contracts come from Rust; presets are shared data, not fixed entity classes.
export type {
  World_Serialize as World, WorldNode as WorldEntity,
  ProjectView_Serialize as ProjectView, Workspace_Serialize as Workspace,
  EditorDraft_Serialize as EditorDraft, NodeTypeDefinition,
} from './bindings';
import type { World, WorldEntity } from './model';
import presets from '../../schema/world-presets.json';
import {defaultProperties} from './node-fields';
export const emptyWorld = (): World => structuredClone(presets) as World;
export function availableNodeTypes(world:World){return [...world.nodeTypes,...presets.nodeTypes.filter(p=>!world.nodeTypes.some(t=>t.id===p.id))];}
export function withBuiltinNodeType(world:World,typeId:string):World {
  if(world.nodeTypes.some(t=>t.id===typeId))return world;
  const preset=presets.nodeTypes.find(t=>t.id===typeId);if(!preset)return world;
  const capabilities=presets.componentTypes.filter(d=>preset.components.includes(d.id)&&!world.componentTypes.some(c=>c.id===d.id));
  const relationships=presets.relationTypes.filter(d=>d.id==='core.holds-claim'&&preset.components.includes('core.claim')&&!world.relationTypes.some(r=>r.id===d.id));
  return {...world,nodeTypes:[...world.nodeTypes,structuredClone(preset)],componentTypes:[...world.componentTypes,...structuredClone(capabilities)],relationTypes:[...world.relationTypes,...structuredClone(relationships)]} as World;
}
export function makeNode(world: World, typeId: string, name: string, id: string = crypto.randomUUID()): WorldEntity {
  const preset = world.nodeTypes.find((d) => d.id === typeId);
  return { id, typeId, name, description: '', visualDescription: '',
    components: Object.fromEntries((preset?.components ?? []).map((id) => {
      const definition = world.componentTypes.find((d) => d.id === id);
      return [id, definition?defaultProperties(definition):{}];
    })) };
}
const id = (n: number) => '10000000-0000-4000-8000-' + String(n).padStart(12, '0');
export function exampleWorld(): World {
  const world = emptyWorld();
  const definitions: [number, string, string, string, string][] = [
    [1,'core.place','Brasswater House','An old country house where the river bends. Someone has returned after a long absence.','Weathered limestone, slate roofs and ivy around tall windows.'],
    [2,'core.place','The glasshouse','Mara tends the last plants her mentor left behind. A sealed letter waits beneath the potting bench.','Green iron ribs, fogged panes, terracotta pots and an oak potting bench.'],
    [3,'core.place','The map room','One of the river charts shows a channel that no longer exists.','Walnut shelves, rolled charts, a worktable and faded green curtains.'],
    [4,'core.character','Mara Vale','An adult herbalist with a dry sense of humor. She wants to learn why her mentor left abruptly.','Thirty-two years old, dark curly hair tied back, brown eyes and a pale scar through her left eyebrow.'],
    [5,'core.item','Waxed field coat','A practical coat with pockets full of old plant labels.','Moss-green waxed cotton, brass buttons, patched elbows and a corduroy collar.'],
    [6,'core.item','Canvas satchel','Mara carries her field supplies here.','Faded ochre canvas, a leather strap and a square brass buckle.'],
    [7,'core.item','Seed packets','Unmarked seeds collected from the riverbank.','Folded brown-paper envelopes tied with blue thread.'],
    [8,'core.thread','The unopened letter','Who left it, and why did they ask Mara to wait until the first frost?',''],
  ];
  world.nodes = definitions.map(([n,typeId,name,description,visualDescription]) => ({...makeNode(world,typeId,name,id(n)),description,visualDescription}));
  world.nodes[4].components['core.wearable'] = {regions:'torso, arms',layer:2};
  world.nodes[4].components['core.item'].unit='coat';
  world.nodes[5].components['core.container'] = {};
  world.nodes[5].components['core.item'].unit='satchel';
  world.nodes[6].components['core.item'] = {quantity:3,unit:'packet'};
  const links: [number,string,number,number][] = [[11,'inside',2,1],[12,'inside',3,1],[13,'present-at',4,2],[15,'stored-in',6,4],[16,'stored-in',7,6],[17,'worn-by',5,4],[18,'related',8,2],[19,'related',8,4]];
  world.edges=links.map(([n,type,source,target])=>({id:id(n),typeId:'core.'+type,sourceId:id(source),targetId:id(target),description:''}));
  return world;
}
