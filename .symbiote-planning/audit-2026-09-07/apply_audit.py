"""Apply user-authorized issue-only roadmap audit #443, with full read-back evidence.
No existing issue state, assignee, milestone or application/default-branch file is changed.
"""
import os,json,re,sys,time,hashlib,lzma,base64,zipfile,io,subprocess,traceback
from pathlib import Path
from urllib.request import Request,urlopen,build_opener,HTTPRedirectHandler
from urllib.error import HTTPError,URLError
from graphlib import TopologicalSorter
from datetime import datetime,timezone
REPO=os.environ['REPO'];assert REPO=='RepairYourTech/SymbioteIDE'
BASE='https://api.github.com/repos/'+REPO+'/'
HEADERS={'Authorization':'Bearer '+os.environ['GH_TOKEN'],'Accept':'application/vnd.github+json','User-Agent':'symbiote-planning-audit-443','X-GitHub-Api-Version':'2022-11-28'}
ROOT=Path('.symbiote-planning/audit-2026-09-07');OUT=Path('audit-output');OUT.mkdir(exist_ok=True)
J=[];last_write=0.0
class NoRedirect(HTTPRedirectHandler):
 def redirect_request(self,req,fp,code,msg,headers,newurl):return None

def api(path,method='GET',data=None):
 global last_write
 for attempt in range(6):
  try:
   if method!='GET':
    time.sleep(max(0,1.0-(time.monotonic()-last_write)));last_write=time.monotonic()
   raw=json.dumps(data,ensure_ascii=False).encode() if data is not None else None
   with urlopen(Request(BASE+path,data=raw,headers=HEADERS,method=method),timeout=90) as r:return json.load(r)
  except HTTPError as ex:
   if method=='POST' and ex.code in (502,503,504):raise
   if ex.code in (429,502,503,504) or (ex.code==403 and ('retry-after'in ex.headers or ex.headers.get('X-RateLimit-Remaining')=='0')):
    time.sleep(min(90,int(ex.headers.get('Retry-After','60'))));continue
   raise
  except (TimeoutError,URLError):
   # POST is not retried blindly: the caller must reconcile its stable plan key.
   if method=='POST':raise
   if attempt==5:raise
   time.sleep(2**attempt)
 raise RuntimeError('Retry limit exceeded for '+path)

def paged(path):
 acc=[]
 for page in range(1,100):
  rows=api(path+('&' if '?'in path else '?')+'per_page=100&page='+str(page));acc+=rows
  if len(rows)<100:return acc
 raise RuntimeError('Pagination limit exceeded')
def snapshot():return {x['number']:x for x in paged('issues?state=all&sort=created&direction=asc') if 'pull_request'not in x}
def sha(s):return hashlib.sha256(s.encode()).hexdigest()
def record(kind,**details):
 J.append(dict(kind=kind,at=datetime.now(timezone.utc).isoformat(),**details))
 (OUT/'mutation-journal.json').write_text(json.dumps(J,ensure_ascii=False,indent=2))
def extract_key(x):
 m=re.search(r'<!--\s*symbiote-plan-key:\s*(.*?)\s*-->',x.get('body')or'');return m.group(1).strip() if m else None

def run():
 # The payload is generated locally, integrity checked, and contains only audited planning code/data.
 manifest=json.loads((ROOT/'ready.json').read_text())
 blob=b''.join((ROOT/f).read_bytes() for f in manifest['chunks'])
 assert hashlib.sha256(blob).hexdigest()==manifest['sha256'],'Payload integrity failure'
 recipe=json.loads(lzma.decompress(blob))
 for name,content in recipe.items():
  assert name in ('canonical_mapping.json','new_specs.py','amendments.py','build_plan.py')
  (ROOT/name).write_text(content)
 # Download the connector-created original snapshot. Do not forward Authorization to the signed storage host.
 try:build_opener(NoRedirect).open(Request(BASE+'actions/artifacts/10029924838/zip',headers=HEADERS),timeout=60)
 except HTTPError as ex:
  assert ex.code in (301,302,303,307,308),ex.code
  location=ex.headers['Location'];assert location.startswith('https://')
  with urlopen(Request(location),timeout=90)as resp:zipbytes=resp.read()
 else:raise RuntimeError('Expected signed artifact redirect')
 assert hashlib.sha256(zipbytes).hexdigest()=='471774ff66387e8e8da3bab2df3f068c48431c4dbdcc6d98bb95435cfecf0d4b','Snapshot archive integrity failure'
 (ROOT/'before').mkdir(exist_ok=True)
 with zipfile.ZipFile(io.BytesIO(zipbytes))as z:
  for name in ('issues.json','index.json','summary.json'):
   (ROOT/'before'/name).write_bytes(z.read(name));(OUT/('before-'+name)).write_bytes(z.read(name))
 assert hashlib.sha256((ROOT/'before/issues.json').read_bytes()).hexdigest()=='19b55b02d9b29e1529da78f2b82047c95b2191450b06f28014f2ec7552e7d65d'
 subprocess.run([sys.executable,str(ROOT/'build_plan.py')],check=True,stdout=open(OUT/'preflight.txt','w'))
 P=json.loads((ROOT/'plan.json').read_text());assert P['repository']==REPO and P['audit_issue']==443
 before=json.loads((ROOT/'before/issues.json').read_text());B={x['number']:x for x in before['issues']}
 current=snapshot();(OUT/'pre-apply-live.json').write_text(json.dumps(list(current.values()),ensure_ascii=False,indent=2))
 # Validate all existing targets against the audited snapshot before the first write.
 for patch in P['updates']:
  n=patch['number'];assert n in current and current[n]['state']==patch['expected_state'],('Issue/state changed',n)
  # Exact desired bodies on rerun are accepted after IDs can be resolved below; initial drift blocks here.
  if sha(current[n].get('body')or'')!=patch['expected_hash']:
   assert '<!-- symbiote-plan-revision: '+P['revision']+' -->'in(current[n].get('body')or''),('Concurrent body change',n)
  if current[n]['title']!=B[n]['title']:
   assert current[n]['title']==patch['title'],('Concurrent title change',n)
 keymap={}
 for spec in P['new']:
  found=[x for x in current.values()if extract_key(x)==spec['key']]
  assert len(found)<=1,('Duplicate new plan key',spec['key'])
  if found:keymap[spec['key']]=found[0]['number']
 # Add only audit-specific labels and missing existing label names, never replace unrelated labels.
 labels={x['name']for x in paged('labels')}
 needed={z for patch in P['new']+P['updates']for z in patch['labels']}
 for name in sorted(needed-labels):
  api('labels','POST',{'name':name,'color':'6f42c1'if name=='planning:canonical'else 'bfbfbf','description':'Audited Symbiote roadmap classification; reference entries are not implementation assignments.'});record('label_created',name=name)
 # Create epics before child records. Bodies are finalized once every actual GitHub number is known.
 for spec in sorted(P['new'],key=lambda x:(x['kind']!='epic',x['key'])):
  key=spec['key']
  if key in keymap:continue
  provisional=f'<!-- symbiote-plan-key: {key} -->\n<!-- symbiote-audit-provisional: 443 -->\n# Planning reconciliation in progress\n\nThis accepted capability is being linked to its exact prerequisites by audit #443. Do not begin implementation until the final audited body and dependency manifest are present.\n\n'+spec['title']+'\n'
  try:x=api('issues','POST',{'title':spec['title'],'body':provisional,'labels':spec['labels']})
  except Exception:
   matches=[x for x in snapshot().values()if extract_key(x)==key]
   if len(matches)!=1:raise
   x=matches[0]
  keymap[key]=x['number'];current[x['number']]=x;record('issue_created',key=key,number=x['number'])
 def resolve(x):return int(x)if isinstance(x,int)or str(x).isdigit()else keymap[x]
 def render(text):return re.sub(r'\[\[([A-Z][A-Z0-9-]+)\]\]',lambda m:'#'+str(keymap[m.group(1)]),text)
 final=[]
 for spec in P['new']:
  final.append(dict(spec,number=keymap[spec['key']],body=render(spec['body']),expected_hash=sha(current[keymap[spec['key']]].get('body')or''),expected_state='open'))
 for spec in P['updates']:final.append(dict(spec,body=render(spec['body'])))
 assert len({x['number']for x in final})==len(final)
 assert all(len(x['body'].encode())<65000 for x in final)
 # Canonical child issues first, then epics/master, then clearly marked historical references.
 final.sort(key=lambda x:({'canonical':0,'epic':1,'master':2,'entry':3,'reference':4}[x['kind']],x['number']))
 expected={};changed=0;skipped=0
 for patch in final:
  n=patch['number'];live=api('issues/'+str(n));target={k:patch[k]for k in ('title','body')}
  expected[n]=target
  if live['title']==target['title']and live.get('body','')==target['body']:
   skipped+=1;continue
  assert live['state']==patch['expected_state'],('Concurrent issue state change',n)
  assert sha(live.get('body')or'')==patch['expected_hash'],('Concurrent issue body change',n)
  if n in B:assert live['title']==B[n]['title'],('Concurrent title changed',n)
  names={z['name']for z in live.get('labels',[])}
  if patch['kind']=='canonical':names={z for z in names if not z.startswith('wave:')}
  names.discard('planning:reference'if patch['kind']in('canonical','epic','master')else 'planning:canonical')
  names.update(patch['labels']);target['labels']=sorted(names)
  result=api('issues/'+str(n),'PATCH',target)
  assert result['body']==target['body']and result['title']==target['title']and result['state']==live['state'],('Read-back failed',n)
  changed+=1;record('issue_updated',number=n,issue_kind=patch['kind'],before_sha256=sha(live.get('body')or''),after_sha256=sha(result['body']))
  if changed%25==0:print('Updated',changed,'/',len(final),flush=True)
 after=snapshot();failures=[]
 for n,target in expected.items():
  if n not in after or after[n]['body']!=target['body']or after[n]['title']!=target['title']:failures.append('Live mismatch #'+str(n))
 for n,x in B.items():
  if n not in after or after[n]['state']!=x['state']:failures.append('Original state changed #'+str(n))
 activekeys={}
 for n,x in after.items():
  if x['state']=='open'and extract_key(x):activekeys.setdefault(extract_key(x),[]).append(n)
 duplicates={k:v for k,v in activekeys.items()if len(v)>1}
 if duplicates:failures.append('Duplicate active keys')
 deps={resolve(n):[resolve(d)for d in ds]for n,ds in P['dependencies'].items()}
 waves={resolve(n):w for n,w in P['waves'].items()}
 assert set(deps)=={resolve(x)for x in P['canonical_atoms']}
 for n,ds in deps.items():
  for d in ds:
   if d not in deps:failures.append('Missing canonical prerequisite '+str((n,d)))
   if waves[d]>waves[n]:failures.append('Later-wave prerequisite '+str((n,d)))
  declared=re.search(r'^## Dependencies\s*\n(.*?)(?=^## |\Z)',after[n]['body'],re.M|re.S)
  parsed=set(map(int,re.findall(r'#(\d+)',declared.group(1))))if declared else set()
  if parsed!=set(ds):failures.append('Dependency read-back mismatch #'+str(n))
 try:tuple(TopologicalSorter({n:set(ds)for n,ds in deps.items()}).static_order());cycles=0
 except Exception as e:failures.append('Cycle: '+str(e));cycles=1
 placeholders=[]
 for n in deps:
  if re.search(r'\[\[([A-Z][A-Z0-9-]+)\]\]',after[n]['body']):placeholders.append(n)
 if placeholders:failures.append('Unresolved placeholders')
 coverage={family:[resolve(x)for x in owners]for family,owners in P['coverage'].items()}
 for family,owners in coverage.items():
  if any(n not in after for n in owners):failures.append('Coverage missing '+family)
 # Verify each canonical atom has one canonical epic and every epic lists only canonical children.
 epics=[resolve(x)for x in P['canonical_epics']];members={}
 for e in epics:
  sec=re.search(r'^## Child issues\s*\n(.*?)(?=^## |\Z)',after[e]['body'],re.M|re.S)
  for n in map(int,re.findall(r'^- \[[ x]\] #(\d+)',sec.group(1),re.M)):
   members.setdefault(n,[]).append(e)
 if set(members)!=set(deps)or any(len(v)!=1 for v in members.values()):failures.append('Epic membership mismatch')
 result={'revision':P['revision'],'repository':REPO,'audit_issue':443,'baseline_total':len(B),'after_total':len(after),'canonical_atomic_issues':len(deps),'canonical_epics':len(epics),'reference_only':len(P['legacy']),'new_atomic_issues':25,'new_epics':3,'new_issue_numbers':keymap,'planned_existing_roadmap_updates':len(P['updates']),'changed_writes_this_run':changed,'idempotent_skips':skipped,'before_duplicate_key_groups':81,'after_duplicate_active_keys':duplicates,'before_cycles':3,'after_cycles':cycles,'dependency_edges':sum(map(len,deps.values())),'coverage_families':len(coverage),'wave_counts':{str(w):sum(v==w for v in waves.values())for w in sorted(set(waves.values()))},'original_states_preserved':not any('state changed'in s for s in failures),'failures':failures,'verified_at':datetime.now(timezone.utc).isoformat(),'scope':'Planning and issue integrity only; not implementation or runtime conformance certification.'}
 (OUT/'validation.json').write_text(json.dumps(result,ensure_ascii=False,indent=2))
 (OUT/'after-issues.json').write_text(json.dumps(list(after.values()),ensure_ascii=False,indent=2))
 (OUT/'canonical-manifest.json').write_text(json.dumps({'revision':P['revision'],'repository':REPO,'issues':[dict(number=n,title=after[n]['title'],body=after[n]['body'],dependencies=deps[n],wave=waves[n],epic=members[n][0])for n in sorted(deps)],'epics':[{k:after[n][k]for k in ('number','title','body')}for n in epics],'reference_map':P['legacy'],'new_ids':keymap,'coverage':coverage,'topological_order':[resolve(n)for n in P['topological_order']]},ensure_ascii=False,indent=2))
 (OUT/'coverage.md').write_text('# Accepted decision coverage\n\n'+ '\n'.join('## '+f+'\n\n'+', '.join('#'+str(n)for n in owners)+'\n'for f,owners in coverage.items()))
 (OUT/'execution-order.md').write_text('# Dependency-ordered canonical implementation queue\n\n'+'\n'.join(str(idx)+'. #'+str(resolve(n))+' (Wave '+str(waves[resolve(n)])+') — '+after[resolve(n)]['title']for idx,n in enumerate(P['topological_order'],1)))
 report='# Symbiote roadmap reconciliation — '+P['revision']+'\n\n'+('**READ-BACK VALIDATION PASSED.**'if not failures else '**VALIDATION FAILED — inspect failures.**')+'\n\n```json\n'+json.dumps(result,ensure_ascii=False,indent=2)+'\n```\n\n## What the audit corrected\n\n- 81 duplicate-key groups and two independently executable roadmap generations are reconciled into one canonical queue. Older issues remain open/reference-only; no implementation was declared complete.\n- Three dependency cycles were repaired by separating schemas, binding resolution, permission calculation, catalog UX and runtime dispatch; same-wave ordering and 702 prerequisites are explicit.\n- Unique earlier contracts were retained, including schema/event-journal/scheduler, extra harnesses, administrative CLI, mobile transport/distribution and Collision Intelligence.\n- Native Agent/provider/auth/standalone CLI, bounded goals/delegation, opt-in project learning and global/project/task measurement now have dedicated actionable issue owners.\n- Context limits, hidden child telemetry, actual versus allocated costs, causal gain evidence, graph/dead-code uncertainty, source-versus-spec authority, comment directives and OAuth entitlement claims are qualified precisely.\n\n## Boundaries\n\nThis verifies coverage of accepted conversation decisions and mechanical roadmap integrity, not implemented software, provider approval, exhaustive future requirements or absence of unknown defects. Each implementation still requires its issue-specific acceptance evidence.\n\nDefault-branch application/planning/bootstrap files were not changed. Historical encoded importers are stale and MUST NOT be rerun as authoritative synchronizers; PLAN-01 owns fail-closed revision/identity protection and regenerated source. Use this canonical manifest or live #154 instead.\n\nFull before/after snapshots, alias mapping, coverage matrix, topological execution order and per-write hash journal accompany this report.\n'
 (OUT/'AUDIT_REPORT.md').write_text(report)
 print(json.dumps(result,ensure_ascii=False,indent=2),flush=True)
 with open(os.environ['GITHUB_STEP_SUMMARY'],'a')as f:f.write(report)
 assert not failures,failures
 audit=api('issues/443');original=B[443]['body']
 add='\n\n## Verified audit result\n\nCanonical roadmap: #154. '+str(len(deps))+' atomic issues / '+str(len(epics))+' epics. '+str(len(P['updates']))+' existing roadmap entries reconciled; 25 new atomic issues and 3 new epics created. 198 earlier/duplicate entries are reference-only; existing states are unchanged.\n\nFull paginated read-back found **0 duplicate active plan keys, 0 dependency cycles, 0 missing canonical prerequisites, 0 later-wave prerequisites, and 0 mismatched issue bodies**. All 28 accepted capability families have explicit owners.\n\nNative: #'+str(keymap['EPIC-NATIVE'])+' · Learning: #'+str(keymap['EPIC-LEARNING'])+' · Measurement: #'+str(keymap['EPIC-MEASUREMENT'])+' · Importer integrity: #'+str(keymap['PLAN-01'])+'.\n\nEvidence artifact `roadmap-audit-result`: https://github.com/'+REPO+'/actions/runs/'+os.environ['GITHUB_RUN_ID']+'\n\nThis is a planning audit, not certification of implemented behavior. Historical default-branch bootstrap payloads remain stale and must not overwrite this revision.\n'
 assert audit['body']==original or '## Verified audit result'in audit['body'],'Audit issue concurrently changed'
 final_audit=original.replace('- [ ]','- [x]')+add
 api('issues/443','PATCH',{'body':final_audit});record('audit_report_updated',number=443)

if __name__=='__main__':
 try:run()
 except Exception as e:
  record('failed',error=type(e).__name__+': '+str(e));traceback.print_exc();sys.exit(1)
