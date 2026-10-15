// Exercise the same pure geometry and selection functions used by the editor.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const source = fs.readFileSync(require('node:path').join(__dirname, '../frontend/app-v2.js'), 'utf8');
const context=vm.createContext({config:{display:{width:480,height:320},widgets:[
  {x:0,y:0,width:30,height:30,group_id:'cpu',locked:false,enabled:true},
  {x:30,y:0,width:30,height:30,group_id:'cpu',locked:false,enabled:true},
  {x:60,y:0,width:30,height:30,group_id:'cpu',locked:true,enabled:true},
  {x:90,y:0,width:30,height:30,locked:false,enabled:false}
]},selectedWidgets:new Set([0]),selectedWidget:0});
vm.runInContext(source.slice(source.indexOf('function snapAxis('),source.indexOf('function startResize(')),context);
assert.equal(context.snapAxis(13,20,[],8,3).position,16);
assert.equal(context.snapAxis(13,20,[35],8,3).position,15,'right edge snaps to guide');
assert.equal(context.snapAxis(13,20,[99],8,3).guide,null);
assert.equal(context.snapAxis(13,20,[],0,3).position,13,'off means no grid snapping');
assert.deepEqual([...context.movableSelection()],[0,1],'groups move together but locks are respected');
assert(!context.snapTargets([0,1,2],'x').includes(90),'hidden widgets do not attract snapping');
assert(context.snapTargets([],'x').includes(240),'display centre is a guide');
console.log('Editor regression passed: grid, edge/centre guides, hidden objects and group locks.');
vm.runInContext(source.slice(source.indexOf('function sensorHealth('),source.indexOf('async function refreshStatus(')),context);
const info={value:0,sampled_at_ms:1000,stale_after_ms:10000};
assert.equal(context.sensorHealth(info,2000).state,'available','zero is a real reading');
assert.equal(context.sensorHealth({...info,value:null},2000).state,'unavailable');
assert.equal(context.sensorHealth(info,12000).state,'stale');
assert.equal(context.sensorHealth({...info,error:'source offline'},2000).state,'stale');
console.log('Sensor health regression passed: zero, missing, stale, source error.');

// New network widgets opt into readable units without migrating saved formats.
vm.runInContext(source.slice(source.indexOf('const defaults ='), source.indexOf('const fonts =')) +
  '\nthis.widgetDefaults = defaults; this.widgetAffixes = defaultAffixes;', context);
for (const kind of ['network_upload', 'network_download']) {
  assert.equal(context.widgetDefaults[kind], '{speed}');
  assert.equal(context.widgetAffixes[kind]?.right || '', '', 'auto unit is not duplicated');
}
assert.equal(context.widgetDefaults.cpu_usage, '{value}', 'other sensor defaults are unchanged');
console.log('Network defaults regression passed: automatic units for new upload/download widgets.');
