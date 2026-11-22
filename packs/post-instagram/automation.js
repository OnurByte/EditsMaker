// Instagram Reels, run inside the signed-in instagram.com page: Instagram's resumable upload (video, then a cover
// frame) and configure_to_clips, the same steps as Tehom's tehom-instagram, made by the web page with its cookies, so
// no password ever passes through edits.
async (p) => {
  const ck = n => (document.cookie.match(new RegExp('(?:^|; )' + n + '=([^;]+)')) || [])[1] || '';
  const uid = ck('ds_user_id');
  if (!uid) throw 'login';
  const H = {'x-ig-app-id': '936619743392459', 'x-csrftoken': ck('csrftoken'), 'x-requested-with': 'XMLHttpRequest', 'x-instagram-ajax': '1'};
  const call = async (path, init = {}) => {
    const r = await fetch(path, {credentials: 'include', ...init, headers: {...H, ...(init.headers || {})}});
    if (r.status === 401 || r.status === 403) throw 'login';
    return r;
  };
  if (p.do === 'who') {
    const v = await (await call(`/api/v1/users/${uid}/info/`)).json().catch(() => ({}));
    return {id: uid, name: (v.user || {}).username || uid};
  }
  if (p.expect && p.expect !== uid) throw 'the browser is signed in to a different Instagram account; run edits connect instagram again';

  const file = document.getElementById('__edits_file').files[0];
  // size, length and a cover frame from the video itself
  const v = document.createElement('video');
  v.muted = true;
  v.src = URL.createObjectURL(file);
  await new Promise((ok, no) => { v.onloadeddata = ok; v.onerror = () => no('the browser could not open the video'); });
  v.currentTime = Math.min(1, v.duration / 2);
  await new Promise(ok => { v.onseeked = ok; });
  const cv = document.createElement('canvas');
  cv.width = v.videoWidth;
  cv.height = v.videoHeight;
  cv.getContext('2d').drawImage(v, 0, 0);
  const cover = await new Promise(ok => cv.toBlob(ok, 'image/jpeg', 0.9));
  const w = v.videoWidth, h = v.videoHeight, ms = Math.round(v.duration * 1000);
  const id = String(Date.now()), name = `fb_uploader_${id}`;
  // www.instagram.com serves the upload paths too; i.instagram.com is the older host
  const upload = async (kind, body, params, type) => {
    for (const host of ['', 'https://i.instagram.com']) {
      const r = await call(`${host}/rupload_${kind}/${name}`, {method: 'POST', body, headers: {
        'x-entity-name': name, 'x-entity-length': String(body.size), 'x-entity-type': type, offset: '0', 'content-type': 'application/octet-stream',
        'x-instagram-rupload-params': JSON.stringify(params)}});
      if (r.ok) return;
      if (r.status !== 404 && !host) throw `Instagram refused the ${kind === 'igvideo' ? 'video' : 'cover'} (HTTP ${r.status}) ${(await r.text()).slice(0, 160)}`;
    }
    throw `Instagram has no upload address for the ${kind === 'igvideo' ? 'video' : 'cover'}`;
  };
  await upload('igvideo', file, {is_clips_video: '1', media_type: 2, upload_id: id, upload_media_width: w, upload_media_height: h, upload_media_duration_ms: ms, video_format: 'video/mp4', 'client-passthrough': '1'}, 'video/mp4');
  await upload('igphoto', cover, {media_type: 2, upload_id: id, upload_media_width: w, upload_media_height: h, image_compression: '{"lib_name":"moz","lib_version":"3.1.m","quality":"90"}'}, 'image/jpeg');
  const form = new URLSearchParams({upload_id: id, caption: p.caption || '', clips_share_preview_to_feed: '1', source_type: 'library', disable_comments: '0', like_and_view_counts_disabled: '0', video_subtitles_enabled: '0'});
  // Instagram transcodes first; configure answers "Transcode not finished yet" until it is done
  for (let i = 0; i < 20; i++) {
    await new Promise(ok => setTimeout(ok, 5000));
    const r = await call('/api/v1/media/configure_to_clips/', {method: 'POST', body: form, headers: {'content-type': 'application/x-www-form-urlencoded'}});
    const t = await r.text();
    if (/transcode not finished/i.test(t) || r.status === 202) continue;
    let a = {};
    try { a = JSON.parse(t); } catch (e) {}
    if (!r.ok || !a.media) throw `Instagram refused the Reel: ${a.message || t.slice(0, 160)} (HTTP ${r.status})`;
    return `https://www.instagram.com/reel/${a.media.code}/`;
  }
  throw 'Instagram took too long to process the video';
}
