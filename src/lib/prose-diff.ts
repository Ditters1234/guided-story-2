export type ProseChange={kind:'same'|'removed'|'added';text:string};

/** Preserve exact whitespace. Trim unchanged edges before a bounded word comparison;
 * very large rewrites become one replacement block rather than an unbounded table. */
export function diffProse(before:string,after:string):ProseChange[]{
  const a=before.match(/\s+|\S+/gu)??[],b=after.match(/\s+|\S+/gu)??[];
  let prefix=0,suffix=0;
  while(prefix<a.length&&prefix<b.length&&a[prefix]===b[prefix])prefix++;
  while(suffix<a.length-prefix&&suffix<b.length-prefix&&a[a.length-1-suffix]===b[b.length-1-suffix])suffix++;
  const left=a.slice(prefix,a.length-suffix),right=b.slice(prefix,b.length-suffix),changes:ProseChange[]=[];
  function push(kind:ProseChange['kind'],text:string){if(!text)return;const last=changes.at(-1);if(last?.kind===kind)last.text+=text;else changes.push({kind,text});}
  push('same',a.slice(0,prefix).join(''));
  if(left.length*right.length>150_000){push('removed',left.join(''));push('added',right.join(''));}
  else{
    const width=right.length+1,table=new Uint32Array((left.length+1)*width);
    for(let i=left.length-1;i>=0;i--)for(let j=right.length-1;j>=0;j--){table[i*width+j]=left[i]===right[j]?1+table[(i+1)*width+j+1]:Math.max(table[(i+1)*width+j],table[i*width+j+1]);}
    let i=0,j=0;
    while(i<left.length||j<right.length){
      if(i<left.length&&j<right.length&&left[i]===right[j]){push('same',left[i++]);j++;}
      else if(i<left.length&&(j===right.length||table[(i+1)*width+j]>=table[i*width+j+1]))push('removed',left[i++]);
      else push('added',right[j++]);
    }
  }
  push('same',a.slice(a.length-suffix).join(''));
  return changes;
}
