// YouTube Shorts, run inside the signed-in studio.youtube.com page: Studio's own resumable upload, then createvideo.
// The steps follow Tehom's tehom-youtube (after adasq/youtube-studio, MIT).
async (p) => {
  const c = k => (window.ytcfg && ytcfg.get && ytcfg.get(k)) || '';
  const ck = n => (document.cookie.match(new RegExp('(?:^|; )' + n + '=([^;]+)')) || [])[1] || '';
  const ch = c('CHANNEL_ID'), key = c('INNERTUBE_API_KEY'), del = c('DELEGATED_SESSION_ID'), ver = c('INNERTUBE_CONTEXT_CLIENT_VERSION') || '1.20201130.03.00';
  const sap = ck('SAPISID') || ck('__Secure-3PAPISID');
  if (location.hostname !== 'studio.youtube.com' || !ch || !sap) throw 'login';
  const enc = new TextEncoder();
  const hex = b => Array.from(new Uint8Array(b), x => x.toString(16).padStart(2, '0')).join('');
  // SAPISIDHASH: the header Google's own web apps send with the cookies
  const H = async (extra = {}) => {
    const t = Math.floor(Date.now() / 1000);
    return {authorization: `SAPISIDHASH ${t}_${hex(await crypto.subtle.digest('SHA-1', enc.encode(`${t} ${sap} https://studio.youtube.com`)))}`, 'x-origin': 'https://studio.youtube.com', 'x-goog-authuser': '0', ...extra};
  };
  const client = {clientName: 62, clientVersion: ver, hl: 'en', gl: 'US'};
  const delegation = {roleType: {channelRoleType: 'CREATOR_CHANNEL_ROLE_TYPE_OWNER'}, externalChannelId: ch};
  if (p.do === 'who') {
    const r = await fetch(`/youtubei/v1/creator/get_creator_channels?alt=json&key=${key}`, {method: 'POST', credentials: 'include', headers: await H({'content-type': 'application/json'}),
      body: JSON.stringify({channelIds: [ch], mask: {channelId: true, title: true}, context: {client, user: {onBehalfOfUser: del}}})});
    const v = await r.json().catch(() => ({}));
    return {id: ch, name: ((v.channels || [])[0] || {}).title || ch};
  }
  if (p.expect && p.expect !== ch) throw 'the browser is signed in to a different YouTube channel; run edits connect youtube again';

  const file = document.getElementById('__edits_file').files[0];
  const id = `innertube_studio:${crypto.randomUUID()}:0`, name = `file-${Date.now()}`, form = 'application/x-www-form-urlencoded;charset=utf-8';
  const start = await fetch('https://upload.youtube.com/upload/studio', {method: 'POST', credentials: 'include',
    headers: await H({'content-type': form, 'x-goog-upload-command': 'start', 'x-goog-upload-file-name': name, 'x-goog-upload-protocol': 'resumable'}),
    body: JSON.stringify({frontendUploadId: id})});
  if (start.status === 401 || start.status === 403) throw 'login';
  const url = start.headers.get('x-goog-upload-url');
  if (!url) throw `YouTube did not start the upload (HTTP ${start.status})`;
  const sent = await fetch(url, {method: 'POST', credentials: 'include',
    headers: await H({'content-type': form, 'x-goog-upload-command': 'upload, finalize', 'x-goog-upload-file-name': name, 'x-goog-upload-offset': '0'}), body: file});
  const res = ((await sent.json().catch(() => ({}))).scottyResourceId) || '';
  if (!res) throw `YouTube did not accept the video (HTTP ${sent.status})`;

  const caption = (p.caption || '').trim();
  const title = Array.from(caption.split('\n')[0]).slice(0, 100).join('') || 'Short';
  const description = caption.includes('#Shorts') ? caption : `${caption}\n\n#Shorts`;
  const r = await fetch(`/youtubei/v1/upload/createvideo?alt=json&key=${key}`, {method: 'POST', credentials: 'include', headers: await H({'content-type': 'application/json'}),
    body: JSON.stringify({
      channelId: ch, resourceId: {scottyResourceId: {id: res}}, frontendUploadId: id,
      initialMetadata: {title: {newTitle: title}, description: {newDescription: description, shouldSegment: true}, privacy: {newPrivacy: p.private ? 'PRIVATE' : 'PUBLIC'}, draftState: {isDraft: false}},
      context: {client, request: {returnLogEntry: true, internalExperimentFlags: []}, user: {onBehalfOfUser: del, delegationContext: delegation, serializedDelegationContext: ''}, clientScreenNonce: ''},
      delegationContext: delegation,
    })});
  const v = await r.json().catch(() => ({}));
  if (!v.videoId) throw `YouTube refused the video: ${(v.error || {}).message || 'no reason given'} (HTTP ${r.status})`;
  return `https://youtube.com/shorts/${v.videoId}`;
}
