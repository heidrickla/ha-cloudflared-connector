'use strict';
// Renders api/status. Every value comes from the status model; this file only lays it out. URLs are
// relative so the page works under Home Assistant's ingress path.

const POLL_MS = 5000;
const STATES = {
  connected: ['good', 'Connected'],
  degraded: ['warn', 'Degraded'],
  starting: ['run', 'Starting'],
  disconnected: ['crit', 'Disconnected'],
  restarting: ['serious', 'Restarting'],
  stopping: ['idle', 'Stopping'],
  unconfigured: ['crit', 'Not configured'],
};
const LOG_FILTERS = [
  ['all', 'All', () => true],
  ['warn', 'Warnings and errors', (l) => ['warn', 'error', 'fatal'].includes(l.level)],
  ['error', 'Errors', (l) => ['error', 'fatal'].includes(l.level)],
];
const LEVEL_TAGS = { debug: 'DBG', info: 'INF', warn: 'WRN', error: 'ERR', fatal: 'FTL', other: '' };

const view = { tab: 'overview', logFilter: 'all', confirmUntil: 0, last: null, failed: false };

// --- small helpers --------------------------------------------------------------------------------

function el(tag, props, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(props || {})) {
    if (v === null || v === undefined || v === false) continue;
    if (k === 'class') e.className = v;
    else if (k === 'style') Object.assign(e.style, v);
    else if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else e.setAttribute(k, v === true ? '' : String(v));
  }
  for (const kid of kids.flat(Infinity)) {
    if (kid === null || kid === undefined || kid === false) continue;
    e.append(kid instanceof Node ? kid : String(kid));
  }
  return e;
}

const dot = (cls, title) => el('span', { class: `dot ${cls}`, title });
const chip = (cls, label, title) => el('span', { class: 'chip', title: title || label }, dot(cls), label);
const card = (title, aside, body, cls) =>
  el('section', { class: `card ${cls || ''}` }, el('h2', {}, el('span', {}, title), aside || null), body);

function duration(s) {
  if (s === null || s === undefined) return '-';
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min`;
  return `${Math.floor(s / 86400)} d ${Math.floor((s % 86400) / 3600)} h`;
}

function bytes(n) {
  if (n === null || n === undefined) return '-';
  const units = ['B', 'kB', 'MB', 'GB', 'TB'];
  let i = 0;
  while (n >= 1000 && i < units.length - 1) { n /= 1000; i += 1; }
  return `${n.toFixed(i === 0 || n >= 100 ? 0 : 1)} ${units[i]}`;
}

const count = (n) => (n === null || n === undefined ? '-' : n.toLocaleString());
const when = (unix) => (unix ? new Date(unix * 1000).toLocaleString([], {
  month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit' }) : '');
const clock = (unix) => (unix ? new Date(unix * 1000).toLocaleString([], {
  month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false }) : '');

function bar(fraction, cls, title) {
  const pct = Math.max(0, Math.min(1, fraction)) * 100;
  return el('div', { class: 'bar', title }, el('i', { class: cls, style: { width: `${pct.toFixed(1)}%` } }));
}

function codeClass(code) {
  return { 1: 's4', 2: 's3', 3: 's1', 4: 'warn', 5: 'crit' }[String(code)[0]] || 'idle';
}

// --- sections -------------------------------------------------------------------------------------

function tiles(s) {
  const [cls, word] = STATES[s.state] || ['idle', s.state];
  const c = s.connections;
  const t = s.traffic;
  const tile = (k, v, vcls, alert) =>
    el('div', { class: `tile${alert ? ' alert' : ''}`, title: k },
      el('div', { class: 'k' }, k), el('div', { class: 'v row' }, vcls ? dot(vcls) : null, v));
  return [
    tile('State', word, cls, cls !== 'good'),
    tile('Edge connections', `${c.ready} of ${c.expected}`,
      c.ready >= c.expected ? 'good' : c.ready > 0 ? 'warn' : 'crit'),
    tile('cloudflared up', duration(s.process.uptime_s), s.process.running ? 'run' : 'idle'),
    tile('Restarts', String(s.process.restarts), s.process.restarts ? 'warn' : 'good'),
    tile('Requests', count(t.requests), null),
    tile('Origin errors', count(t.errors), t.errors ? 'warn' : 'good'),
    tile('cloudflared', s.cloudflared.version || '-', null),
  ];
}

function connectionsCard(s) {
  const links = s.connections.links;
  const maxRtt = Math.max(100, ...links.map((l) => l.rtt_ms || 0));
  const rows = links.map((l) => {
    const state = l.up === true ? ['good', 'up'] : l.up === false ? ['crit', 'down'] : ['idle', 'unknown'];
    const head = el('div', { class: 'head' },
      dot(state[0], state[1]), el('b', {}, `Connection ${l.index}`),
      l.location ? chip('s1', l.location.toUpperCase(), 'Cloudflare edge location') : null,
      l.protocol ? chip('s4', l.protocol, 'transport protocol') : null,
      el('span', { class: 'small muted', title: when(l.since) },
        l.for_s === null ? state[1] : `${state[1]} for ${duration(l.for_s)}`));
    const rtt = el('div', { class: 'metric' }, el('span', { class: 'muted' }, 'Round trip'),
      bar((l.rtt_ms || 0) / maxRtt, 's3', 'smoothed round-trip time to the edge'),
      el('span', { class: 'val' }, l.rtt_ms ? `${l.rtt_ms} ms` : '-'));
    const io = el('div', { class: 'legend' },
      el('span', {}, 'sent ', el('b', {}, bytes(l.sent_bytes))),
      el('span', {}, 'received ', el('b', {}, bytes(l.received_bytes))));
    const err = l.error && l.up === false ? el('div', { class: 'small muted' }, `Last error: ${l.error}`) : null;
    return el('div', { class: 'link' }, head, rtt, io, err);
  });
  const body = rows.length ? el('div', { class: 'links' }, rows)
    : el('p', { class: 'muted' }, s.process.running ? 'No connection registered yet.' : 'cloudflared is not running.');
  return card('Edge connections', el('span', { class: 'small muted' }, `${s.connections.ready} of ${s.connections.expected} registered`), body, 'wide');
}

function trafficCard(s) {
  const t = s.traffic;
  const total = t.responses.reduce((n, r) => n + r.count, 0);
  const big = el('div', { class: 'big' },
    el('div', {}, el('b', {}, count(t.requests)), el('span', {}, 'requests')),
    el('div', {}, el('b', {}, count(t.errors)), el('span', {}, 'origin errors')),
    el('div', {}, el('b', {}, count(t.concurrent)), el('span', {}, 'in flight')));
  const responses = total
    ? [el('div', { class: 'stack' }, t.responses.map((r) =>
        el('i', { class: codeClass(r.code), style: { flex: String(r.count) }, title: `${r.code}: ${r.count.toLocaleString()}` }))),
      el('div', { class: 'legend' }, t.responses.map((r) =>
        el('span', {}, dot(codeClass(r.code)), r.code, ' ', el('b', {}, r.count.toLocaleString()))))]
    : el('p', { class: 'small muted' }, 'No responses counted yet.');
  return card('Traffic', el('span', { class: 'small muted' }, 'since cloudflared started'),
    [big, el('div', { class: 'small muted label' }, 'Responses by status code'), responses]);
}

function connectorCard(s) {
  const p = s.process;
  const cf = s.cloudflared;
  const facts = [
    ['cloudflared', cf.version ? `${cf.version}${cf.go_version ? ` (${cf.go_version})` : ''}` : '-'],
    ['Connector ID', cf.connector_id ? el('code', {}, cf.connector_id) : '-'],
    ['Configuration', cf.config_version === null ? 'not received' : `version ${cf.config_version}`],
    ['Log level', el('code', {}, cf.loglevel)],
    ['Add-on up', duration(p.connector_uptime_s)],
    ['Restarts', String(p.restarts)],
    ['Last exit', p.last_exit_text ? `${p.last_exit_text}, ${when(p.last_exit.at)}` : 'none'],
  ];
  const list = el('dl', { class: 'facts' }, facts.map(([k, v]) => [el('dt', {}, k), el('dd', {}, v)]));
  const confirming = Date.now() < view.confirmUntil;
  const button = el('button', {
    type: 'button', class: confirming ? 'danger' : null, disabled: !p.can_restart,
    onclick: () => restart(confirming),
  }, confirming ? 'Confirm restart' : 'Restart cloudflared');
  const note = el('span', { class: 'small muted' },
    confirming ? 'Every connection through the tunnel drops for a few seconds, this page too if it came through the tunnel.'
      : p.can_restart ? '' : 'Needs a tunnel token.');
  return card('Connector', null, [list, el('div', { class: 'actions' }, button, note)]);
}

function hostnamesCard(s) {
  const r = s.routes;
  if (!r.received) {
    return card('Public hostnames', null, el('p', { class: 'muted' },
      s.process.running ? 'cloudflared has not received its configuration from the dashboard yet.' : 'cloudflared is not running.'), 'wide');
  }
  const rows = r.hostnames.map((h) => el('tr', {},
    el('td', {}, h.hostname), el('td', {}, h.path ? el('code', {}, h.path) : el('span', { class: 'muted' }, 'any')),
    el('td', {}, el('code', {}, h.service))));
  if (r.fallback) {
    rows.push(el('tr', {}, el('td', { class: 'muted' }, 'anything else'), el('td', {}, ''),
      el('td', {}, el('code', {}, r.fallback))));
  }
  const table = el('table', {}, el('tr', {}, el('th', {}, 'Hostname'), el('th', {}, 'Path'), el('th', {}, 'Service')), rows);
  const aside = el('span', { class: 'small muted' }, `${r.hostnames.length} rules, configuration version ${s.cloudflared.config_version}`);
  return card('Public hostnames', aside, [table,
    el('p', { class: 'small muted' }, 'Set in the Zero Trust dashboard and read from the configuration cloudflared is running. Rules match top to bottom.')], 'wide');
}

function logCard(s) {
  const filter = LOG_FILTERS.find(([k]) => k === view.logFilter) || LOG_FILTERS[0];
  const lines = s.log.filter(filter[2]);
  const buttons = el('div', { class: 'filters' }, LOG_FILTERS.map(([k, label]) =>
    el('button', { type: 'button', 'aria-pressed': String(k === view.logFilter),
      onclick: () => { view.logFilter = k; render(); } }, label)));
  const pre = el('div', { class: 'log', id: 'logbox' }, lines.map((l) =>
    el('div', {}, el('time', { title: when(l.at) }, clock(l.at)),
      el('span', { class: `lvl lvl-${l.level}` }, LEVEL_TAGS[l.level] || ''), el('span', { class: 'text', title: l.text }, l.message))));
  const aside = el('span', { class: 'small muted' }, `${lines.length} of the last ${s.log.length} lines`);
  return card('cloudflared log', aside, [buttons, lines.length ? pre : el('p', { class: 'muted' }, 'No lines.')], 'wide');
}

// --- page -----------------------------------------------------------------------------------------

function render() {
  const s = view.last;
  if (!s) return;
  document.getElementById('version').textContent = s.addon_version;
  document.getElementById('tiles').replaceChildren(...tiles(s));
  document.getElementById('headline').textContent = s.headline;
  const notices = s.notices.map((n) => el('div', { class: 'notice small' }, n));
  if (view.failed) notices.unshift(el('div', { class: 'notice small' }, 'The panel lost contact with the connector; showing the last status.'));
  document.getElementById('notices').replaceChildren(...notices);

  const box = document.getElementById('logbox');
  const atBottom = !box || box.scrollTop + box.clientHeight >= box.scrollHeight - 8;
  document.getElementById('tab-overview').replaceChildren(
    connectionsCard(s), el('div', { class: 'pair' }, trafficCard(s), connectorCard(s)));
  document.getElementById('tab-hostnames').replaceChildren(hostnamesCard(s));
  document.getElementById('tab-log').replaceChildren(logCard(s));
  const newBox = document.getElementById('logbox');
  if (newBox && atBottom) newBox.scrollTop = newBox.scrollHeight;
  else if (newBox && box) newBox.scrollTop = box.scrollTop;

  for (const b of document.querySelectorAll('.tabs button')) {
    const on = b.dataset.tab === view.tab;
    b.setAttribute('aria-selected', String(on));
    document.getElementById(`tab-${b.dataset.tab}`).hidden = !on;
  }
  document.getElementById('foot').textContent = `Status at ${when(s.generated_at)}. Refreshes every ${POLL_MS / 1000} s.`;
}

async function refresh() {
  try {
    const r = await fetch('api/status', { cache: 'no-store' });
    if (!r.ok) throw new Error(String(r.status));
    view.last = await r.json();
    view.failed = false;
  } catch (e) {
    view.failed = true;
  }
  render();
}

async function restart(confirmed) {
  if (!confirmed) {
    view.confirmUntil = Date.now() + 4000;
    render();
    setTimeout(render, 4100);
    return;
  }
  view.confirmUntil = 0;
  try {
    await fetch('api/restart', { method: 'POST', headers: { 'X-Connector-Action': 'restart' } });
  } finally {
    setTimeout(refresh, 1500);
  }
}

function setTab(tab) {
  view.tab = ['overview', 'hostnames', 'log'].includes(tab) ? tab : 'overview';
  render();
  if (view.tab === 'log') {
    const box = document.getElementById('logbox');
    if (box) box.scrollTop = box.scrollHeight;
  }
}

function theme(next) {
  document.documentElement.dataset.theme = next;
  document.getElementById('theme').textContent = next === 'dark' ? 'Light' : 'Dark';
  try { localStorage.setItem('cloudflared-panel-theme', next); } catch (e) { /* storage blocked */ }
}

let saved = null;
try { saved = localStorage.getItem('cloudflared-panel-theme'); } catch (e) { /* storage blocked */ }
theme(saved === 'light' ? 'light' : 'dark');
document.getElementById('theme').addEventListener('click', () =>
  theme(document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark'));
for (const b of document.querySelectorAll('.tabs button')) {
  b.addEventListener('click', () => { history.replaceState(null, '', `#${b.dataset.tab}`); setTab(b.dataset.tab); });
}
view.tab = location.hash.slice(1) || 'overview';
refresh().then(() => setTab(view.tab));
setInterval(refresh, POLL_MS);
