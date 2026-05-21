// ── Face metadata builder ────────────────────────────────────────────────
//
//  Decomposes a kernel mesh into logical faces by grouping triangles first by
//  raw `face_id`, then sub-dividing each group by quantized normal direction.
//  For a rectangular extruded solid this yields 6 logical faces
//  (top + bottom + 4 side walls) even though the kernel produces only 3
//  raw face groups (top cap=1, bottom cap=2, all side walls=3).
//
//  Input  : geometry result { positions, normals, face_ids, indices }
//  Output : faces[] = [{
//             face_id:          number,   // logical id (1..N)
//             source_face_id:   number,   // kernel face_id
//             normal:           [x,y,z],
//             center:           [x,y,z],
//             triangle_indices: number[], // flat (3 entries per triangle)
//             vertex_count:     number,
//           }]
//
//  API:
//    window.CadInteraction.picking.buildFaceMetadata(result) → faces[]

pub const JS: &str = r##"
(function registerFaceMetadata() {
  window.CadInteraction.picking = window.CadInteraction.picking || {};

  window.CadInteraction.picking.buildFaceMetadata = function(result) {
    if (!result || !result.positions || !result.indices) return [];

    var pos  = result.positions;
    var nrm  = result.normals  || [];
    var fids = result.face_ids || [];
    var idx  = result.indices;
    var nTri = Math.floor(idx.length / 3);

    // Step 1: group triangles by raw face_id
    var byFaceId = new Map();
    for (var t = 0; t < nTri; t++) {
      var i0  = idx[t * 3];
      var fid = fids[i0] !== undefined ? fids[i0] : 1;
      if (!byFaceId.has(fid)) byFaceId.set(fid, []);
      byFaceId.get(fid).push(t);
    }

    // Step 2: for each raw group, sub-divide by quantized normal
    var logicalFaces = [];
    var counter = 1;

    byFaceId.forEach(function(triList, sourceFid) {
      var subGroups = new Map();

      for (var k = 0; k < triList.length; k++) {
        var t = triList[k];
        var nx = 0, ny = 0, nz = 0;
        for (var vi = 0; vi < 3; vi++) {
          var vIdx = idx[t * 3 + vi];
          nx += (nrm[vIdx * 3]     || 0);
          ny += (nrm[vIdx * 3 + 1] || 0);
          nz += (nrm[vIdx * 3 + 2] || 0);
        }
        var nl = Math.hypot(nx, ny, nz) || 1;
        nx /= nl; ny /= nl; nz /= nl;
        var key = nx.toFixed(1) + ',' + ny.toFixed(1) + ',' + nz.toFixed(1);
        if (!subGroups.has(key)) {
          subGroups.set(key, { tris: [], nx: 0, ny: 0, nz: 0, cnt: 0 });
        }
        var sg = subGroups.get(key);
        sg.tris.push(t);
        sg.nx += nx; sg.ny += ny; sg.nz += nz; sg.cnt++;
      }

      subGroups.forEach(function(sg) {
        var nl  = Math.hypot(sg.nx, sg.ny, sg.nz) || 1;
        var fnx = sg.nx / nl, fny = sg.ny / nl, fnz = sg.nz / nl;

        var cx = 0, cy = 0, cz = 0, vcount = 0;
        for (var k = 0; k < sg.tris.length; k++) {
          var t = sg.tris[k];
          for (var vi = 0; vi < 3; vi++) {
            var vIdx = idx[t * 3 + vi];
            cx += pos[vIdx * 3];
            cy += pos[vIdx * 3 + 1];
            cz += pos[vIdx * 3 + 2];
            vcount++;
          }
        }
        if (vcount > 0) { cx /= vcount; cy /= vcount; cz /= vcount; }

        var triFlat = new Array(sg.tris.length * 3);
        for (var k = 0; k < sg.tris.length; k++) {
          var t = sg.tris[k];
          triFlat[k * 3]     = idx[t * 3];
          triFlat[k * 3 + 1] = idx[t * 3 + 1];
          triFlat[k * 3 + 2] = idx[t * 3 + 2];
        }

        logicalFaces.push({
          face_id:          counter,
          source_face_id:   sourceFid,
          normal:           [fnx, fny, fnz],
          center:           [cx, cy, cz],
          triangle_indices: triFlat,
          vertex_count:     vcount,
        });
        counter++;
      });
    });

    // Collect topology summary for debug log
    var nFaces = logicalFaces.length;
    var nTrisTotal = Math.floor(idx.length / 3);
    // Unique vertices in output (rough: count unique positions in indices)
    var nEdgesEst = nTrisTotal * 3 / 2 | 0;  // Euler: E = F*3/2 for closed manifold approx
    console.log('[FaceMetadata] faces=' + nFaces +
      ' edges~' + nEdgesEst +
      ' vertices=' + (result.positions.length / 3 | 0) +
      ' triangleToFace=' + nTrisTotal);

    return logicalFaces;
  };

  console.log('[CadInteraction.picking] face metadata ready');
})();
"##;
