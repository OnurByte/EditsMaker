// TikTok, run inside the signed-in tiktok.com/tiktokstudio page: its own scripts sign the site's requests. The steps
// follow Tehom's tehom-tiktok (TikTok's web uploader: auth, project, AWS-signed VOD upload, post).
async (p) => {
  const AID = '1988';
  const j = async (method, url, headers = {}, body) => {
    const r = await fetch(url, {method, headers: {accept: 'application/json, text/plain, */*', ...headers}, body, credentials: 'include'});
    const t = await r.text();
    let v = null;
    try { v = JSON.parse(t); } catch (e) {}
    if (r.status === 401 || r.status === 403) throw 'login';
    if (r.status !== 200) throw `TikTok refused ${url.split('?')[0].split('/').slice(3).join('/')} (HTTP ${r.status}) ${t.slice(0, 160)}`;
    return v || {};
  };
  const who = await j('GET', `https://www.tiktok.com/passport/web/account/info/?aid=${AID}`);
  const d = who.data || {};
  const uid = String(d.user_id || '');
  if (!uid || uid === '0') throw 'login';
  if (p.do === 'who') return {id: uid, name: d.username || d.screen_name || uid};
  if (p.expect && p.expect !== uid) throw 'the browser is signed in to a different TikTok account; run edits connect tiktok again';

  const file = document.getElementById('__edits_file').files[0];
  const enc = new TextEncoder();
  const hex = b => Array.from(new Uint8Array(b), x => x.toString(16).padStart(2, '0')).join('');
  const sha = async s => hex(await crypto.subtle.digest('SHA-256', enc.encode(s)));
  const hmac = async (k, s) => crypto.subtle.sign('HMAC', await crypto.subtle.importKey('raw', typeof k === 'string' ? enc.encode(k) : k, {name: 'HMAC', hash: 'SHA-256'}, false, ['sign']), enc.encode(s));

  const tok = (await j('GET', `https://www.tiktok.com/api/v1/video/upload/auth/?aid=${AID}`)).video_token_v5 || {};
  // the misspelling is TikTok's
  const ak = tok.access_key_id, sk = tok.secret_acess_key, st = tok.session_token || '';
  if (!ak || !sk) throw 'login';
  const abc = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_';
  const creation = Array.from(crypto.getRandomValues(new Uint8Array(21)), b => abc[b % abc.length]).join('');
  const created = await j('POST', `https://www.tiktok.com/api/v1/web/project/create/?creation_id=${creation}&type=1&aid=${AID}`);
  if (!created.project) throw 'login';

  // the upload service: AWS Signature V4 with the temporary keys TikTok just gave
  const vod = async (method, query, body) => {
    const date = new Date().toISOString().replace(/[-:]/g, '').replace(/\.\d+/, '');
    const day = date.slice(0, 8), region = 'ap-singapore-1';
    const q = Object.entries(query).sort(([a], [b]) => (a < b ? -1 : 1)).map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`).join('&');
    const ph = await sha(body);
    const h = [];
    if (body) h.push(['x-amz-content-sha256', ph]);
    h.push(['x-amz-date', date], ['x-amz-security-token', st]);
    const signed = h.map(([k]) => k).join(';');
    const req = `${method}\n/top/v1\n${q}\n${h.map(([k, v]) => `${k}:${v.trim()}\n`).join('')}\n${signed}\n${ph}`;
    const scope = `${day}/${region}/vod/aws4_request`;
    let key = await hmac('AWS4' + sk, day);
    for (const part of [region, 'vod', 'aws4_request']) key = await hmac(key, part);
    const headers = Object.fromEntries(h);
    headers.authorization = `AWS4-HMAC-SHA256 Credential=${ak}/${scope}, SignedHeaders=${signed}, Signature=${hex(await hmac(key, `AWS4-HMAC-SHA256\n${date}\n${scope}\n${await sha(req)}`))}`;
    if (body) headers['content-type'] = 'text/plain;charset=UTF-8';
    return j(method, `https://www.tiktok.com/top/v1?${q}`, headers, body || undefined);
  };
  const applied = await vod('GET', {Action: 'ApplyUploadInner', Version: '2020-11-19', SpaceName: 'tiktok', FileType: 'video', IsInner: '1', FileSize: String(file.size), s: 'g158iqx8434'}, '');
  const node = ((applied.Result || {}).InnerUploadAddress || {}).UploadNodes?.[0] || {};
  const store = (node.StoreInfos || [])[0] || {};
  if (!node.UploadHost || !store.StoreUri || !node.Vid) throw 'TikTok offered no upload address';

  // the bytes go to TikTok's storage host in 5 MB parts, each with its CRC32
  const table = Array.from({length: 256}, (_, n) => { for (let k = 0; k < 8; k++) n = n & 1 ? 0xedb88320 ^ (n >>> 1) : n >>> 1; return n >>> 0; });
  const crc32 = a => { let c = -1; for (const b of a) c = table[(c ^ b) & 255] ^ (c >>> 8); return ((c ^ -1) >>> 0).toString(16).padStart(8, '0'); };
  const up = `https://${node.UploadHost}/${store.StoreUri}`, uploadId = crypto.randomUUID(), CHUNK = 5 * 1024 * 1024, parts = [];
  for (let off = 0, n = 1; off < file.size; off += CHUNK, n++) {
    const chunk = new Uint8Array(await file.slice(off, off + CHUNK).arrayBuffer());
    const c = crc32(chunk);
    const r = await fetch(`${up}?partNumber=${n}&uploadID=${uploadId}&phase=transfer`, {method: 'POST', headers: {Authorization: store.Auth, 'Content-Type': 'application/octet-stream', 'Content-Disposition': 'attachment; filename="undefined"', 'Content-Crc32': c}, body: chunk});
    if (!r.ok) throw `TikTok storage refused part ${n} (HTTP ${r.status})`;
    parts.push(`${n}:${c}`);
  }
  const fin = await fetch(`${up}?uploadID=${uploadId}&phase=finish&uploadmode=part`, {method: 'POST', headers: {Authorization: store.Auth, 'Content-Type': 'text/plain;charset=UTF-8'}, body: parts.join(',')});
  if (!fin.ok) throw `TikTok storage did not finish the upload (HTTP ${fin.status})`;
  await vod('POST', {Action: 'CommitUploadInner', Version: '2020-11-19', SpaceName: 'tiktok'}, JSON.stringify({SessionKey: node.SessionKey, Functions: [{name: 'GetMeta'}]}));

  // #hashtags marked up the way TikTok's web uploader does, offsets in UTF-16 units
  const caption = p.caption || '', cs = Array.from(caption), extra = [];
  let markup = '', off = 0;
  for (let i = 0; i < cs.length;) {
    if (cs[i] === '#') {
      let e = i + 1;
      while (e < cs.length && /[\p{L}\p{N}_]/u.test(cs[e])) e++;
      if (e > i + 1) {
        const tag = cs.slice(i, e).join(''), id = extra.length;
        extra.push({start: off, end: off + tag.length, hashtag_name: tag.slice(1), tag_id: String(id), type: 1, user_id: ''});
        markup += `<h id="${id}">${tag}</h>`;
        off += tag.length;
        i = e;
        continue;
      }
    }
    markup += cs[i];
    off += cs[i].length;
    i++;
  }
  const post = {
    post_common_info: {creation_id: creation, enter_post_page_from: 1, post_type: 3},
    feature_common_info_list: [{
      geofencing_regions: [], playlist_name: '', playlist_id: '', tcm_params: '{"commerce_toggle_info":{}}', sound_exemption: 0, anchors: [],
      vedit_common_info: {draft: '', video_id: node.Vid},
      privacy_setting_info: {visibility_type: p.private ? 1 : 0, allow_duet: 0, allow_stitch: 0, allow_comment: 1},
    }],
    single_post_req_list: [{batch_index: 0, video_id: node.Vid, is_long_video: 0,
      single_post_feature_info: {text: caption, text_extra: extra, markup_text: markup, music_info: {}, poster_delay: 0}}],
  };
  const r = await fetch(`https://www.tiktok.com/tiktok/web/project/post/v1/?app_name=tiktok_web&channel=tiktok_web&device_platform=web&aid=${AID}`,
    {method: 'POST', credentials: 'include', headers: {'content-type': 'application/json', accept: 'application/json, text/plain, */*'}, body: JSON.stringify(post)});
  const v = await r.json().catch(() => ({}));
  if (r.status !== 200 || v.status_code !== 0) throw `TikTok rejected the post: ${v.status_msg || 'no reason given'} (HTTP ${r.status})`;
  const item = String(((v.single_post_resp_list || [])[0] || {}).item_id || '');
  return item ? `https://www.tiktok.com/@${d.username}/video/${item}` : 'posted';
}
