"""Read-only issue snapshot for user-authorized roadmap audit #443."""
import os, json, re, time, hashlib
from pathlib import Path
from urllib.request import Request, urlopen
from datetime import datetime, timezone
repo = os.environ['REPO']
assert repo == 'RepairYourTech/SymbioteIDE'
headers = {'Authorization': 'Bearer '+os.environ['GH_TOKEN'], 'Accept':'application/vnd.github+json', 'User-Agent':'symbiote-roadmap-audit'}
def get(path):
    for attempt in range(4):
        try:
            with urlopen(Request('https://api.github.com/repos/'+repo+'/'+path, headers=headers), timeout=60) as r:
                return json.load(r)
        except Exception:
            if attempt == 3: raise
            time.sleep(2**attempt)
def pages(path):
    data=[]
    for page in range(1,100):
        rows=get(path+('&' if '?' in path else '?')+'per_page=100&page='+str(page))
        data.extend(rows)
        if len(rows)<100: return data
    raise RuntimeError('Pagination cap exceeded')
issues=[x for x in pages('issues?state=all&sort=created&direction=asc') if 'pull_request' not in x]
comments={str(x['number']): pages('issues/'+str(x['number'])+'/comments') for x in issues if x.get('comments',0)}
out=Path('audit-output');out.mkdir(exist_ok=True)
snapshot={'repository':repo,'captured_at':datetime.now(timezone.utc).isoformat(),'issues':issues,'comments':comments}
(out/'issues.json').write_text(json.dumps(snapshot,ensure_ascii=False,indent=2))
index=[]; keys={}
for x in issues:
    body=x.get('body') or ''
    m=re.search(r'<!--\s*symbiote-plan-key:\s*([^>]+?)\s*-->',body)
    key=m.group(1).strip() if m else None
    if key: keys.setdefault(key,[]).append(x['number'])
    index.append({'number':x['number'],'key':key,'title':x['title'],'state':x['state'],'comments':x.get('comments',0),'updated_at':x['updated_at'],'body_sha256':hashlib.sha256(body.encode()).hexdigest()})
(out/'index.json').write_text(json.dumps(index,ensure_ascii=False,indent=2))
summary={'total_issues':len(issues),'open':sum(x['state']=='open' for x in issues),'managed':sum(x['key'] is not None for x in index),'duplicate_keys':{k:v for k,v in keys.items() if len(v)>1},'comments_on_issues':list(comments)}
(out/'summary.json').write_text(json.dumps(summary,indent=2))
print(json.dumps(summary,indent=2))
with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:
    f.write('# Roadmap snapshot\n\n```json\n'+json.dumps(summary,indent=2)+'\n```\n')
