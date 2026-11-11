import { describe, expect, it } from 'vitest';
import { libraryDragData, readLibraryDrag, libraryPositions } from './node-library';
import { exampleWorld } from './model';
import type { LibraryEntry_Serialize } from './bindings';

describe('portable library map interactions',()=>{
  it('drags a pinned file identity rather than trusting node JSON from a drag payload',()=>{
    const file={fileName:'traveller.json',sha256:'a'.repeat(64),name:'Traveller',description:'',nodeCount:2,connectionCount:1,error:null};
    expect(readLibraryDrag(libraryDragData(file))).toEqual({fileName:file.fileName,sha256:file.sha256});
    for(const invalid of ['', '{}', '{broken', JSON.stringify({fileName:'x',sha256:'bad'}),JSON.stringify({fileName:5,sha256:'a'.repeat(64)})])expect(readLibraryDrag(invalid)).toBeNull();
  });
  it('places the entire group at the drop point while preserving relative saved positions',()=>{
    const world=exampleWorld(),positions=Object.fromEntries(world.nodes.map((node,index)=>[node.id,{x:500+index*280,y:600+index*120}]));
    const entry:LibraryEntry_Serialize={schemaVersion:1,name:'Fictional group',description:'',world,positions};
    const before=structuredClone(entry),placed=libraryPositions(entry,{x:50,y:80});
    expect(placed[world.nodes[0].id]).toEqual({x:50,y:80});
    expect(placed[world.nodes[1].id]).toEqual({x:330,y:200});
    expect(entry).toEqual(before);
  });
  it('uses the ordinary hierarchy layout for hand-authored JSON without map positions',()=>{
    const world=exampleWorld();
    const positions=libraryPositions({schemaVersion:1,name:'Group',description:'',world},{x:100,y:200});
    expect(Object.keys(positions)).toHaveLength(world.nodes.length);
    expect(Math.min(...Object.values(positions).map(p=>p.x!))).toBe(100);
    expect(Math.min(...Object.values(positions).map(p=>p.y!))).toBe(200);
  });
});
