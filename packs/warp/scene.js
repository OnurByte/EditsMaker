// Warp: stars streaking past as at light speed, out from the middle of the frame; each star makes whole laps of the
// depth per loop, so it loops.
(THREE, scene, camera, opts) => {
  const count = 600, seconds = 4, depth = 40;
  let s = 3;
  const rand = () => ((s = (s * 16807) % 2147483647) / 2147483647);
  const stars = [];
  const pos = new Float32Array(count * 6);
  for (let i = 0; i < count; i++) {
    const a = rand() * Math.PI * 2, r = 0.4 + rand() * 6;
    stars.push({ x: Math.cos(a) * r, y: Math.sin(a) * r * 1.6, z: rand() * depth, laps: 1 + Math.floor(rand() * 2) });
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  const mat = new THREE.LineBasicMaterial({ color: 0xdfe8ff, transparent: true, opacity: 0.8, blending: THREE.AdditiveBlending });
  scene.add(new THREE.LineSegments(geo, mat));
  return (t) => {
    const k = (t % seconds) / seconds;
    for (let i = 0; i < count; i++) {
      const st = stars[i];
      const z = -((st.z + k * depth * st.laps) % depth) - 1;
      // a streak: longer the nearer it is
      const len = Math.min(4, 30 / -z);
      pos.set([st.x, st.y, z, st.x, st.y, Math.min(z + len, -0.5)], i * 6);
    }
    geo.attributes.position.needsUpdate = true;
  };
}
