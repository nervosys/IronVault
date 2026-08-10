//! Embedded web dashboard for IronVault.
//!
//! Single-page HTML application served at the root path.

/// Return the complete dashboard HTML as a static string.
pub fn dashboard_html() -> &'static str {
    r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>IronVault</title>
<style>
:root{--bg:#0f1117;--card:#1a1d27;--border:#2a2d3a;--text:#e1e4ed;--muted:#8b8fa3;--accent:#6366f1;--accent-hover:#818cf8;--green:#22c55e;--red:#ef4444;--orange:#f59e0b}
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:var(--bg);color:var(--text);min-height:100vh}
.header{background:var(--card);border-bottom:1px solid var(--border);padding:1rem 2rem;display:flex;align-items:center;justify-content:space-between}
.header h1{font-size:1.25rem;font-weight:600}
.header h1 span{color:var(--accent)}
.badge{font-size:.7rem;padding:.15rem .5rem;border-radius:999px;background:var(--green);color:#000;font-weight:600;margin-left:.5rem}
.badge.off{background:var(--red);color:#fff}
.container{max-width:1200px;margin:0 auto;padding:1.5rem}
.login{max-width:400px;margin:4rem auto;background:var(--card);border:1px solid var(--border);border-radius:.75rem;padding:2rem}
.login h2{margin-bottom:1rem}
.login input{width:100%;padding:.6rem .75rem;border:1px solid var(--border);border-radius:.375rem;background:var(--bg);color:var(--text);margin-bottom:1rem;font-size:.9rem}
.login button,.btn{background:var(--accent);color:#fff;border:none;padding:.5rem 1.25rem;border-radius:.375rem;cursor:pointer;font-size:.875rem;font-weight:500}
.login button:hover,.btn:hover{background:var(--accent-hover)}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:1rem;margin-bottom:1.5rem}
.card{background:var(--card);border:1px solid var(--border);border-radius:.75rem;padding:1.25rem}
.card h3{font-size:1rem;margin-bottom:.5rem}
.card .value{font-size:1.75rem;font-weight:700;color:var(--accent)}
.card .label{font-size:.75rem;color:var(--muted);text-transform:uppercase;letter-spacing:.05em}
table{width:100%;border-collapse:collapse;margin-top:.75rem}
th,td{text-align:left;padding:.5rem .75rem;border-bottom:1px solid var(--border);font-size:.85rem}
th{color:var(--muted);font-weight:500;text-transform:uppercase;font-size:.7rem;letter-spacing:.05em}
.tabs{display:flex;gap:0;border-bottom:1px solid var(--border);margin-bottom:1.25rem}
.tab{padding:.5rem 1.25rem;cursor:pointer;color:var(--muted);border-bottom:2px solid transparent;font-size:.875rem}
.tab.active{color:var(--accent);border-bottom-color:var(--accent)}
.section{display:none}.section.active{display:block}
.mono{font-family:'SF Mono',Consolas,monospace;font-size:.8rem}
.empty{color:var(--muted);text-align:center;padding:2rem}
#error{color:var(--red);font-size:.85rem;margin-bottom:.5rem;display:none}
.topbar-actions{display:flex;align-items:center;gap:1rem}
</style>
</head>
<body>

<div class="header">
  <h1><span>&#9670;</span> IronVault</h1>
  <div class="topbar-actions">
    <span id="status" class="badge off">Locked</span>
    <button class="btn" id="logoutBtn" style="display:none" onclick="logout()">Logout</button>
  </div>
</div>

<div class="container">
  <!-- login -->
  <div id="loginView" class="login">
    <h2>Unlock Vault</h2>
    <div id="error"></div>
    <input type="password" id="passphrase" placeholder="Vault passphrase" autocomplete="off"/>
    <button onclick="login()">Unlock</button>
  </div>

  <!-- dashboard -->
  <div id="dashView" style="display:none">
    <div class="grid" id="statsGrid"></div>
    <div class="tabs">
      <div class="tab active" onclick="switchTab('models',this)">Models</div>
      <div class="tab" onclick="switchTab('audit',this)">Audit Log</div>
      <div class="tab" onclick="switchTab('conversions',this)">Conversions</div>
    </div>
    <div id="models" class="section active"></div>
    <div id="audit" class="section"></div>
    <div id="conversions" class="section"></div>
  </div>
</div>

<script>
const API='/api/v1';
let TOKEN='';

function el(id){return document.getElementById(id)}
function showErr(m){const e=el('error');e.textContent=m;e.style.display='block'}
function hideErr(){el('error').style.display='none'}

async function apiFetch(path,opts={}){
  opts.headers=opts.headers||{};
  if(TOKEN)opts.headers['Authorization']='Bearer '+TOKEN;
  const r=await fetch(API+path,opts);
  if(!r.ok){const b=await r.json().catch(()=>({error:r.statusText}));throw new Error(b.error||r.statusText)}
  return r;
}

async function login(){
  hideErr();
  const pp=el('passphrase').value;
  if(!pp){showErr('Enter passphrase');return}
  try{
    const r=await fetch(API+'/auth/token',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({passphrase:pp})});
    if(!r.ok){const b=await r.json().catch(()=>({}));throw new Error(b.error||'Auth failed')}
    const d=await r.json();TOKEN=d.token;
    el('loginView').style.display='none';el('dashView').style.display='block';
    el('status').textContent='Unlocked';el('status').classList.remove('off');
    el('logoutBtn').style.display='inline-block';
    loadDashboard();
  }catch(e){showErr(e.message)}
}

function logout(){
  TOKEN='';el('loginView').style.display='';el('dashView').style.display='none';
  el('status').textContent='Locked';el('status').classList.add('off');
  el('logoutBtn').style.display='none';el('passphrase').value='';
}

async function loadDashboard(){
  try{
    const [stats,models]=await Promise.all([
      apiFetch('/stats').then(r=>r.json()),
      apiFetch('/models').then(r=>r.json())
    ]);
    el('statsGrid').innerHTML=`
      <div class="card"><div class="label">Models</div><div class="value">${stats.model_count}</div></div>
      <div class="card"><div class="label">Versions</div><div class="value">${stats.total_versions}</div></div>
      <div class="card"><div class="label">Storage</div><div class="value">${formatSize(stats.total_size_bytes)}</div></div>
      <div class="card"><div class="label">Files</div><div class="value">${stats.file_count}</div></div>`;
    renderModels(models);
    loadAudit();
    loadConversions();
  }catch(e){console.error(e)}
}

function renderModels(models){
  if(!models.length){el('models').innerHTML='<div class="empty">No models stored yet.</div>';return}
  let h='<table><thead><tr><th>Name</th><th>Versions</th><th>Actions</th></tr></thead><tbody>';
  for(const m of models){
    h+=`<tr><td>${esc(m.name)}</td><td>${m.version_count}</td><td><button class="btn" onclick="viewVersions('${esc(m.name)}')">Versions</button></td></tr>`;
  }
  h+='</tbody></table>';el('models').innerHTML=h;
}

async function viewVersions(name){
  try{
    const vs=await apiFetch('/models/'+encodeURIComponent(name)+'/versions').then(r=>r.json());
    let h=`<h3 style="margin-bottom:.75rem">Versions of ${esc(name)}</h3><button class="btn" style="margin-bottom:.75rem" onclick="loadDashboard()">&#8592; Back</button>`;
    h+='<table><thead><tr><th>Ver</th><th>Format</th><th>Size</th><th>Timestamp</th><th>Checksum</th></tr></thead><tbody>';
    for(const v of vs){
      h+=`<tr><td>${v.version}</td><td>${esc(v.format)}</td><td>${formatSize(v.size_bytes)}</td><td class="mono">${v.timestamp}</td><td class="mono">${v.checksum_sha256.slice(0,12)}…</td></tr>`;
    }
    h+='</tbody></table>';el('models').innerHTML=h;
  }catch(e){console.error(e)}
}

async function loadAudit(){
  try{
    const entries=await apiFetch('/audit?limit=100').then(r=>r.json());
    if(!entries.length){el('audit').innerHTML='<div class="empty">No audit entries.</div>';return}
    let h='<table><thead><tr><th>Time</th><th>Event</th><th>Description</th><th>Model</th></tr></thead><tbody>';
    for(const e of entries.slice().reverse()){
      h+=`<tr><td class="mono">${e.timestamp}</td><td>${esc(e.event_type)}</td><td>${esc(e.description)}</td><td>${e.model_name||'—'}</td></tr>`;
    }
    h+='</tbody></table>';el('audit').innerHTML=h;
  }catch(e){el('audit').innerHTML='<div class="empty">Could not load audit log.</div>'}
}

async function loadConversions(){
  try{
    const data=await apiFetch('/conversions').then(r=>r.json());
    let h='<table><thead><tr><th>Converter</th><th>From</th><th>To</th></tr></thead><tbody>';
    for(const c of data){h+=`<tr><td>${esc(c.name)}</td><td>${esc(c.source)}</td><td>${esc(c.target)}</td></tr>`}
    h+='</tbody></table>';el('conversions').innerHTML=h;
  }catch(e){el('conversions').innerHTML='<div class="empty">Could not load conversions.</div>'}
}

function switchTab(id,tab){
  document.querySelectorAll('.tab').forEach(t=>t.classList.remove('active'));
  document.querySelectorAll('.section').forEach(s=>s.classList.remove('active'));
  tab.classList.add('active');document.getElementById(id).classList.add('active');
}

function formatSize(b){
  if(b<1024)return b+' B';
  if(b<1048576)return (b/1024).toFixed(1)+' KiB';
  if(b<1073741824)return (b/1048576).toFixed(1)+' MiB';
  return (b/1073741824).toFixed(2)+' GiB';
}

function esc(s){return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;')}

el('passphrase').addEventListener('keydown',e=>{if(e.key==='Enter')login()});
</script>
</body>
</html>"##
}
