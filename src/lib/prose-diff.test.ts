import {describe,expect,it} from 'vitest';
import {diffProse} from './prose-diff';
describe('prose revision comparison',()=>{
  it('shows separated edits while retaining exact punctuation, Unicode and whitespace',()=>{
    const before='Mara waited.\n\n雨 fell; Jon listened.  Still quiet.',after='Mara paused.\n\n雨 fell; Jon watched.  Still quiet.';
    const result=diffProse(before,after);
    expect(result.filter(c=>c.kind!=='added').map(c=>c.text).join('')).toBe(before);
    expect(result.filter(c=>c.kind!=='removed').map(c=>c.text).join('')).toBe(after);
    expect(result.filter(c=>c.kind==='removed').map(c=>c.text)).toEqual(['waited.','listened.']);
    expect(result.filter(c=>c.kind==='added').map(c=>c.text)).toEqual(['paused.','watched.']);
  });
  it('handles empty, identical and whitespace-only edits without dropping text',()=>{
    for(const [before,after] of [['',''],['','A scene.'],['A scene.',''],['Same.','Same.'],['A\n\nB','A B']]){
      const result=diffProse(before,after);expect(result.filter(c=>c.kind!=='added').map(c=>c.text).join('')).toBe(before);expect(result.filter(c=>c.kind!=='removed').map(c=>c.text).join('')).toBe(after);
    }
  });
  it('bounds a large rewrite and keeps both complete versions reconstructable',()=>{
    const before='Old passage. '.repeat(7000),after='New wording. '.repeat(7000),result=diffProse(before,after);
    expect(result.length).toBeLessThanOrEqual(4);expect(result.filter(c=>c.kind!=='added').map(c=>c.text).join('')).toBe(before);expect(result.filter(c=>c.kind!=='removed').map(c=>c.text).join('')).toBe(after);
  });
});
