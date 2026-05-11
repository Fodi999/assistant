// ── Extrude geometry module ──────────────────────────────────────────────
// Domain: Geometry. Pure functions, no GPU access, no DOM access.
// Inputs: SketchState { points, rectangles, circles, dimensions, plane, closed }
// Outputs: { positions, normals, indices, triangleFaceIds, topology }
//
// Designed to be replaced by a Rust backend (truck / OpenCASCADE) later.
// All public entry points live on window.* so other JS fragments and a future
// backend bridge can reach them.

pub const JS: &str = r##"
      // ─── Earcut polygon triangulation (minified, MIT, mapbox/earcut v2.2.4) ───
      // Returns flat array of triangle vertex indices.
      // Input: vertices flat array [x0,y0, x1,y1, ...], holes[], dim (2|3)
      const earcut = (function() {
        'use strict';
        function earcut(data, holeIndices, dim) {
          dim = dim || 2;
          var hasHoles = holeIndices && holeIndices.length,
              outerLen = hasHoles ? holeIndices[0] * dim : data.length,
              outerNode = linkedList(data, 0, outerLen, dim, true),
              triangles = [];
          if (!outerNode || outerNode.next === outerNode.prev) return triangles;
          var minX, minY, maxX, maxY, x, y, invSize;
          if (hasHoles) outerNode = eliminateHoles(data, holeIndices, outerNode, dim);
          if (data.length > 80 * dim) {
            minX = maxX = data[0]; minY = maxY = data[1];
            for (var i = dim; i < outerLen; i += dim) {
              x = data[i]; y = data[i + 1];
              if (x < minX) minX = x; if (y < minY) minY = y;
              if (x > maxX) maxX = x; if (y > maxY) maxY = y;
            }
            invSize = Math.max(maxX - minX, maxY - minY);
            invSize = invSize !== 0 ? 32767 / invSize : 0;
          }
          earcutLinked(outerNode, triangles, dim, minX, minY, invSize, 0);
          return triangles;
        }
        function linkedList(data, start, end, dim, clockwise) {
          var i, last;
          if (clockwise === (signedArea(data, start, end, dim) > 0)) {
            for (i = start; i < end; i += dim) last = insertNode(i, data[i], data[i + 1], last);
          } else {
            for (i = end - dim; i >= start; i -= dim) last = insertNode(i, data[i], data[i + 1], last);
          }
          if (last && equals(last, last.next)) { removeNode(last); last = last.next; }
          return last;
        }
        function filterPoints(start, end) {
          if (!start) return start;
          if (!end) end = start;
          var p = start, again;
          do {
            again = false;
            if (!p.steiner && (equals(p, p.next) || area(p.prev, p, p.next) === 0)) {
              removeNode(p);
              p = end = p.prev;
              if (p === p.next) break;
              again = true;
            } else { p = p.next; }
          } while (again || p !== end);
          return end;
        }
        function earcutLinked(ear, triangles, dim, minX, minY, invSize, pass) {
          if (!ear) return;
          if (!pass && invSize) indexCurve(ear, minX, minY, invSize);
          var stop = ear, prev, next;
          while (ear.prev !== ear.next) {
            prev = ear.prev; next = ear.next;
            if (invSize ? isEarHashed(ear, minX, minY, invSize) : isEar(ear)) {
              triangles.push(prev.i / dim | 0, ear.i / dim | 0, next.i / dim | 0);
              removeNode(ear);
              ear = next.next; stop = next.next;
              continue;
            }
            ear = next;
            if (ear === stop) {
              if (!pass) earcutLinked(filterPoints(ear), triangles, dim, minX, minY, invSize, 1);
              else if (pass === 1) {
                ear = cureLocalIntersections(filterPoints(ear), triangles, dim);
                earcutLinked(ear, triangles, dim, minX, minY, invSize, 2);
              } else if (pass === 2) splitEarcut(ear, triangles, dim, minX, minY, invSize);
              break;
            }
          }
        }
        function isEar(ear) {
          var a = ear.prev, b = ear, c = ear.next;
          if (area(a, b, c) >= 0) return false;
          var ax=a.x,bx=b.x,cx=c.x,ay=a.y,by=b.y,cy=c.y;
          var x0=ax<bx?(ax<cx?ax:cx):(bx<cx?bx:cx);
          var y0=ay<by?(ay<cy?ay:cy):(by<cy?by:cy);
          var x1=ax>bx?(ax>cx?ax:cx):(bx>cx?bx:cx);
          var y1=ay>by?(ay>cy?ay:cy):(by>cy?by:cy);
          var p = c.next;
          while (p !== a) {
            if (p.x >= x0 && p.x <= x1 && p.y >= y0 && p.y <= y1 &&
                pointInTriangle(ax,ay,bx,by,cx,cy,p.x,p.y) &&
                area(p.prev, p, p.next) >= 0) return false;
            p = p.next;
          }
          return true;
        }
        function isEarHashed(ear, minX, minY, invSize) {
          var a=ear.prev,b=ear,c=ear.next;
          if (area(a,b,c) >= 0) return false;
          var ax=a.x,bx=b.x,cx=c.x,ay=a.y,by=b.y,cy=c.y;
          var x0=ax<bx?(ax<cx?ax:cx):(bx<cx?bx:cx);
          var y0=ay<by?(ay<cy?ay:cy):(by<cy?by:cy);
          var x1=ax>bx?(ax>cx?ax:cx):(bx>cx?bx:cx);
          var y1=ay>by?(ay>cy?ay:cy):(by>cy?by:cy);
          var minZ=zOrder(x0,y0,minX,minY,invSize), maxZ=zOrder(x1,y1,minX,minY,invSize);
          var p=ear.prevZ, n=ear.nextZ;
          while (p && p.z >= minZ && n && n.z <= maxZ) {
            if (p.x>=x0&&p.x<=x1&&p.y>=y0&&p.y<=y1&&p!==a&&p!==c&&
                pointInTriangle(ax,ay,bx,by,cx,cy,p.x,p.y)&&area(p.prev,p,p.next)>=0) return false;
            p = p.prevZ;
            if (n.x>=x0&&n.x<=x1&&n.y>=y0&&n.y<=y1&&n!==a&&n!==c&&
                pointInTriangle(ax,ay,bx,by,cx,cy,n.x,n.y)&&area(n.prev,n,n.next)>=0) return false;
            n = n.nextZ;
          }
          while (p && p.z >= minZ) {
            if (p.x>=x0&&p.x<=x1&&p.y>=y0&&p.y<=y1&&p!==a&&p!==c&&
                pointInTriangle(ax,ay,bx,by,cx,cy,p.x,p.y)&&area(p.prev,p,p.next)>=0) return false;
            p = p.prevZ;
          }
          while (n && n.z <= maxZ) {
            if (n.x>=x0&&n.x<=x1&&n.y>=y0&&n.y<=y1&&n!==a&&n!==c&&
                pointInTriangle(ax,ay,bx,by,cx,cy,n.x,n.y)&&area(n.prev,n,n.next)>=0) return false;
            n = n.nextZ;
          }
          return true;
        }
        function cureLocalIntersections(start, triangles, dim) {
          var p = start;
          do {
            var a = p.prev, b = p.next.next;
            if (!equals(a,b) && intersects(a,p,p.next,b) && locallyInside(a,b) && locallyInside(b,a)) {
              triangles.push(a.i/dim|0, p.i/dim|0, b.i/dim|0);
              removeNode(p); removeNode(p.next);
              p = start = b;
            }
            p = p.next;
          } while (p !== start);
          return filterPoints(p);
        }
        function splitEarcut(start, triangles, dim, minX, minY, invSize) {
          var a = start;
          do {
            var b = a.next.next;
            while (b !== a.prev) {
              if (a.i !== b.i && isValidDiagonal(a, b)) {
                var c = splitPolygon(a, b);
                a = filterPoints(a, a.next); c = filterPoints(c, c.next);
                earcutLinked(a, triangles, dim, minX, minY, invSize, 0);
                earcutLinked(c, triangles, dim, minX, minY, invSize, 0);
                return;
              }
              b = b.next;
            }
            a = a.next;
          } while (a !== start);
        }
        function eliminateHoles(data, holeIndices, outerNode, dim) {
          var queue = [], i, len, start, end, list;
          for (i = 0, len = holeIndices.length; i < len; i++) {
            start = holeIndices[i] * dim;
            end = i < len - 1 ? holeIndices[i + 1] * dim : data.length;
            list = linkedList(data, start, end, dim, false);
            if (list === list.next) list.steiner = true;
            queue.push(getLeftmost(list));
          }
          queue.sort(function(a, b) { return a.x - b.x; });
          for (i = 0; i < queue.length; i++) outerNode = eliminateHole(queue[i], outerNode);
          return outerNode;
        }
        function eliminateHole(hole, outerNode) {
          var bridge = findHoleBridge(hole, outerNode);
          if (!bridge) return outerNode;
          var bridgeReverse = splitPolygon(bridge, hole);
          filterPoints(bridgeReverse, bridgeReverse.next);
          return filterPoints(bridge, bridge.next);
        }
        function findHoleBridge(hole, outerNode) {
          var p = outerNode, hx = hole.x, hy = hole.y, qx = -Infinity, m;
          do {
            if (hy <= p.y && hy >= p.next.y && p.next.y !== p.y) {
              var x = p.x + (hy - p.y) * (p.next.x - p.x) / (p.next.y - p.y);
              if (x <= hx && x > qx) { qx = x; m = p.x < p.next.x ? p : p.next; if (x === hx) return m; }
            }
            p = p.next;
          } while (p !== outerNode);
          if (!m) return null;
          var stop = m, mx = m.x, my = m.y, tanMin = Infinity, tan;
          p = m;
          do {
            if (hx >= p.x && p.x >= mx && hx !== p.x &&
                pointInTriangle(hy < my ? hx : qx, hy, mx, my, hy < my ? qx : hx, hy, p.x, p.y)) {
              tan = Math.abs(hy - p.y) / (hx - p.x);
              if (locallyInside(p, hole) && (tan < tanMin || (tan === tanMin && (p.x > m.x || (p.x === m.x && sectorContainsSector(m, p)))))) {
                m = p; tanMin = tan;
              }
            }
            p = p.next;
          } while (p !== stop);
          return m;
        }
        function sectorContainsSector(m, p) { return area(m.prev, m, p.prev) < 0 && area(p.next, m, m.next) < 0; }
        function indexCurve(start, minX, minY, invSize) {
          var p = start;
          do {
            if (p.z === 0) p.z = zOrder(p.x, p.y, minX, minY, invSize);
            p.prevZ = p.prev; p.nextZ = p.next; p = p.next;
          } while (p !== start);
          p.prevZ.nextZ = null; p.prevZ = null;
          sortLinked(p);
        }
        function sortLinked(list) {
          var i, p, q, e, tail, numMerges, pSize, qSize, inSize = 1;
          do {
            p = list; list = null; tail = null; numMerges = 0;
            while (p) {
              numMerges++; q = p; pSize = 0;
              for (i = 0; i < inSize; i++) { pSize++; q = q.nextZ; if (!q) break; }
              qSize = inSize;
              while (pSize > 0 || (qSize > 0 && q)) {
                if (pSize !== 0 && (qSize === 0 || !q || p.z <= q.z)) { e = p; p = p.nextZ; pSize--; }
                else { e = q; q = q.nextZ; qSize--; }
                if (tail) tail.nextZ = e; else list = e;
                e.prevZ = tail; tail = e;
              }
              p = q;
            }
            tail.nextZ = null; inSize *= 2;
          } while (numMerges > 1);
          return list;
        }
        function zOrder(x, y, minX, minY, invSize) {
          x = (x - minX) * invSize | 0; y = (y - minY) * invSize | 0;
          x = (x | (x << 8)) & 0x00FF00FF; x = (x | (x << 4)) & 0x0F0F0F0F;
          x = (x | (x << 2)) & 0x33333333; x = (x | (x << 1)) & 0x55555555;
          y = (y | (y << 8)) & 0x00FF00FF; y = (y | (y << 4)) & 0x0F0F0F0F;
          y = (y | (y << 2)) & 0x33333333; y = (y | (y << 1)) & 0x55555555;
          return x | (y << 1);
        }
        function getLeftmost(start) {
          var p = start, leftmost = start;
          do { if (p.x < leftmost.x || (p.x === leftmost.x && p.y < leftmost.y)) leftmost = p; p = p.next; } while (p !== start);
          return leftmost;
        }
        function pointInTriangle(ax,ay,bx,by,cx,cy,px,py) {
          return (cx-px)*(ay-py)>=(ax-px)*(cy-py) &&
                 (ax-px)*(by-py)>=(bx-px)*(ay-py) &&
                 (bx-px)*(cy-py)>=(cx-px)*(by-py);
        }
        function isValidDiagonal(a, b) {
          return a.next.i !== b.i && a.prev.i !== b.i && !intersectsPolygon(a, b) &&
                 (locallyInside(a, b) && locallyInside(b, a) && middleInside(a, b) &&
                  (area(a.prev, a, b.prev) || area(a, b.prev, b)) ||
                  equals(a, b) && area(a.prev, a, a.next) > 0 && area(b.prev, b, b.next) > 0);
        }
        function area(p, q, r) { return (q.y - p.y) * (r.x - q.x) - (q.x - p.x) * (r.y - q.y); }
        function equals(p1, p2) { return p1.x === p2.x && p1.y === p2.y; }
        function intersects(p1, q1, p2, q2) {
          var o1 = sign(area(p1, q1, p2)), o2 = sign(area(p1, q1, q2));
          var o3 = sign(area(p2, q2, p1)), o4 = sign(area(p2, q2, q1));
          if (o1 !== o2 && o3 !== o4) return true;
          if (o1 === 0 && onSegment(p1, p2, q1)) return true;
          if (o2 === 0 && onSegment(p1, q2, q1)) return true;
          if (o3 === 0 && onSegment(p2, p1, q2)) return true;
          if (o4 === 0 && onSegment(p2, q1, q2)) return true;
          return false;
        }
        function onSegment(p, q, r) {
          return q.x <= Math.max(p.x, r.x) && q.x >= Math.min(p.x, r.x) &&
                 q.y <= Math.max(p.y, r.y) && q.y >= Math.min(p.y, r.y);
        }
        function sign(num) { return num > 0 ? 1 : num < 0 ? -1 : 0; }
        function intersectsPolygon(a, b) {
          var p = a;
          do {
            if (p.i !== a.i && p.next.i !== a.i && p.i !== b.i && p.next.i !== b.i &&
                intersects(p, p.next, a, b)) return true;
            p = p.next;
          } while (p !== a);
          return false;
        }
        function locallyInside(a, b) {
          return area(a.prev, a, a.next) < 0
            ? area(a, b, a.next) >= 0 && area(a, a.prev, b) >= 0
            : area(a, b, a.prev) < 0 || area(a, a.next, b) < 0;
        }
        function middleInside(a, b) {
          var p = a, inside = false, px = (a.x + b.x) / 2, py = (a.y + b.y) / 2;
          do {
            if (((p.y > py) !== (p.next.y > py)) && p.next.y !== p.y &&
                (px < (p.next.x - p.x) * (py - p.y) / (p.next.y - p.y) + p.x)) inside = !inside;
            p = p.next;
          } while (p !== a);
          return inside;
        }
        function splitPolygon(a, b) {
          var a2 = new Node(a.i, a.x, a.y), b2 = new Node(b.i, b.x, b.y);
          var an = a.next, bp = b.prev;
          a.next = b; b.prev = a; a2.next = an; an.prev = a2;
          b2.next = a2; a2.prev = b2; bp.next = b2; b2.prev = bp;
          return b2;
        }
        function insertNode(i, x, y, last) {
          var p = new Node(i, x, y);
          if (!last) { p.prev = p; p.next = p; }
          else { p.next = last.next; p.prev = last; last.next.prev = p; last.next = p; }
          return p;
        }
        function removeNode(p) {
          p.next.prev = p.prev; p.prev.next = p.next;
          if (p.prevZ) p.prevZ.nextZ = p.nextZ;
          if (p.nextZ) p.nextZ.prevZ = p.prevZ;
        }
        function Node(i, x, y) {
          this.i = i; this.x = x; this.y = y;
          this.prev = null; this.next = null;
          this.z = 0; this.prevZ = null; this.nextZ = null;
          this.steiner = false;
        }
        function signedArea(data, start, end, dim) {
          var sum = 0;
          for (var i = start, j = end - dim; i < end; i += dim) {
            sum += (data[j] - data[i]) * (data[i + 1] + data[j + 1]);
            j = i;
          }
          return sum;
        }
        return earcut;
      })();
      window.__earcut = earcut;

      // ─── Plane helpers: 3D <-> 2D projection on sketch plane ──────────────
      // XY plane: uses (x,y), extrudes along +Z
      // XZ plane: uses (x,z), extrudes along +Y  (default)
      // YZ plane: uses (y,z), extrudes along +X
      function getPlaneAxes(plane) {
        if (plane === 'XY') return { u:'x', v:'y', n:[0,0,1], normAxis:'z' };
        if (plane === 'YZ') return { u:'y', v:'z', n:[1,0,0], normAxis:'x' };
        return                { u:'x', v:'z', n:[0,1,0], normAxis:'y' }; // XZ
      }
      window.projectPointToSketch2D = function(point, plane) {
        const ax = getPlaneAxes(plane || 'XZ');
        return [point[ax.u], point[ax.v]];
      };
      window.unprojectSketch2DTo3D = function(point2D, plane, offset) {
        const ax = getPlaneAxes(plane || 'XZ');
        offset = offset || 0;
        const out = { x:0, y:0, z:0 };
        out[ax.u] = point2D[0];
        out[ax.v] = point2D[1];
        out[ax.normAxis] = offset;
        return out;
      };

      // ─── Profile extraction: get active closed profile points (3D) ─────────
      // Returns an array of {x,y,z} in CCW order on the sketch plane,
      // or null if there is no valid closed profile.
      window.getActiveProfilePoints = function(sk) {
        sk = sk || (typeof sketchState !== 'undefined' ? sketchState : null);
        if (!sk) return null;
        const plane = sk.plane || 'XZ';
        const ax = getPlaneAxes(plane);

        // 1. Closed polyline
        if (sk.closed && Array.isArray(sk.points) && sk.points.length >= 3) {
          return sk.points.map(p => ({ x:p.x, y:p.y, z:p.z }));
        }
        // 2. First rectangle (auto-closes)
        if (Array.isArray(sk.rectangles) && sk.rectangles.length > 0) {
          const r = sk.rectangles[0];
          const a = { x:r.x1, y:r.y1, z:r.z1 }, c = { x:r.x2, y:r.y2, z:r.z2 };
          // Build 4 corners on plane by mixing axes
          const make = (u, v) => {
            const o = { x:0, y:0, z:0 };
            o[ax.u] = u; o[ax.v] = v; o[ax.normAxis] = a[ax.normAxis];
            return o;
          };
          return [
            make(a[ax.u], a[ax.v]),
            make(c[ax.u], a[ax.v]),
            make(c[ax.u], c[ax.v]),
            make(a[ax.u], c[ax.v]),
          ];
        }
        // 3. First circle (64 segments by default)
        if (Array.isArray(sk.circles) && sk.circles.length > 0) {
          const c = sk.circles[0];
          const segs = 64;
          const pts = [];
          const cu = c[ax.u + 'c'] !== undefined ? c[ax.u + 'c']
                    : (c['c' + ax.u] !== undefined ? c['c' + ax.u]
                    : ({ cx:c.cx, cy:c.cy, cz:c.cz })[ax.u === 'x' ? 'cx' : ax.u === 'y' ? 'cy' : 'cz']);
          const cv = ({ cx:c.cx, cy:c.cy, cz:c.cz })[ax.v === 'x' ? 'cx' : ax.v === 'y' ? 'cy' : 'cz'];
          const cn = ({ cx:c.cx, cy:c.cy, cz:c.cz })[ax.normAxis === 'x' ? 'cx' : ax.normAxis === 'y' ? 'cy' : 'cz'];
          for (let i = 0; i < segs; i++) {
            const t = (i / segs) * Math.PI * 2;
            const o = { x:0, y:0, z:0 };
            o[ax.u] = cu + Math.cos(t) * c.r;
            o[ax.v] = cv + Math.sin(t) * c.r;
            o[ax.normAxis] = cn;
            pts.push(o);
          }
          return pts;
        }
        return null;
      };

      // ─── 2D signed area (shoelace) ─────────────────────────────────────────
      function signedArea2D(pts2) {
        let s = 0;
        for (let i = 0; i < pts2.length; i++) {
          const a = pts2[i], b = pts2[(i + 1) % pts2.length];
          s += a[0] * b[1] - b[0] * a[1];
        }
        return s * 0.5;
      }

      // ─── MAIN: generate extruded mesh from sketch ──────────────────────────
      // Architecture: pure function. No GPU, no DOM. Easy to swap for backend.
      //
      // Returns:
      // {
      //   positions:        Float32Array,
      //   normals:          Float32Array,
      //   indices:          Uint32Array,
      //   triangleFaceIds:  string[],   // one per triangle
      //   topology: {
      //     faces:    [{ id, kind, triangleStart, triangleCount, normal, area? }],
      //     edges:    [{ id, a, b }],
      //     vertices: [{ id, x, y, z }],
      //   },
      //   meta: { profileCount, depth, plane, direction }
      // }
      window.generateExtrudedMeshFromSketch = function(sk, depth, opts) {
        opts = opts || {};
        sk = sk || (typeof sketchState !== 'undefined' ? sketchState : null);
        if (!sk) throw new Error('generateExtrudedMeshFromSketch: no sketchState');
        if (!(depth > 0)) depth = 1.0;

        const profile3D = window.getActiveProfilePoints(sk);
        if (!profile3D || profile3D.length < 3) {
          throw new Error('generateExtrudedMeshFromSketch: no closed profile (need >=3 pts)');
        }
        const plane = sk.plane || 'XZ';
        const ax = getPlaneAxes(plane);
        const dir = opts.direction || ax.n;

        // 1. Project profile to 2D on plane
        let pts2 = profile3D.map(p => [p[ax.u], p[ax.v]]);

        // 2. Enforce CCW winding (positive signed area)
        if (signedArea2D(pts2) < 0) {
          pts2 = pts2.slice().reverse();
        }
        const N = pts2.length;

        // 3. Earcut triangulation (cap)
        const flat = new Array(N * 2);
        for (let i = 0; i < N; i++) { flat[i*2] = pts2[i][0]; flat[i*2+1] = pts2[i][1]; }
        const capTris = earcut(flat, [], 2); // [i0,i1,i2, ...] indices into pts2
        if (capTris.length === 0) {
          throw new Error('earcut: failed to triangulate profile');
        }

        // 4. Compute 3D positions
        // Plane offset = profile3D[0][normAxis] (sketch may sit at y=0 typically)
        const planeOffset = profile3D[0][ax.normAxis] || 0;
        const baseAt = (i) => {
          const o = { x:0, y:0, z:0 };
          o[ax.u] = pts2[i][0]; o[ax.v] = pts2[i][1]; o[ax.normAxis] = planeOffset;
          return o;
        };
        const topAt = (i) => {
          const b = baseAt(i);
          return { x: b.x + dir[0]*depth, y: b.y + dir[1]*depth, z: b.z + dir[2]*depth };
        };

        const positions = [];
        const normals   = [];
        const indices   = [];
        const triFaceIds = [];

        // Helper to push a triangle
        function pushTri(a, b, c, nx, ny, nz, faceId) {
          const base = positions.length / 3;
          const va = a, vb = b, vc = c;
          positions.push(va.x, va.y, va.z, vb.x, vb.y, vb.z, vc.x, vc.y, vc.z);
          normals.push(nx, ny, nz, nx, ny, nz, nx, ny, nz);
          indices.push(base, base+1, base+2);
          triFaceIds.push(faceId);
        }

        // 5. BASE CAP (facing -dir)
        const baseFaceId = 'face_base';
        const baseN = [-dir[0], -dir[1], -dir[2]];
        const baseTriStart = indices.length / 3;
        for (let t = 0; t < capTris.length; t += 3) {
          // Reverse winding so normal faces -dir
          const i0 = capTris[t], i1 = capTris[t+1], i2 = capTris[t+2];
          pushTri(baseAt(i0), baseAt(i2), baseAt(i1), baseN[0], baseN[1], baseN[2], baseFaceId);
        }
        const baseTriCount = (indices.length / 3) - baseTriStart;

        // 6. TOP CAP (facing +dir)
        const topFaceId = 'face_top';
        const topTriStart = indices.length / 3;
        for (let t = 0; t < capTris.length; t += 3) {
          const i0 = capTris[t], i1 = capTris[t+1], i2 = capTris[t+2];
          pushTri(topAt(i0), topAt(i1), topAt(i2), dir[0], dir[1], dir[2], topFaceId);
        }
        const topTriCount = (indices.length / 3) - topTriStart;

        // 7. SIDE WALLS — one quad per edge i→i+1
        const sideFaces = [];
        for (let i = 0; i < N; i++) {
          const j = (i + 1) % N;
          const a = baseAt(i), b = baseAt(j), c = topAt(j), d = topAt(i);
          // Edge vector & wall normal = edge × extrudeDir (then normalize)
          const ex = b.x - a.x, ey = b.y - a.y, ez = b.z - a.z;
          let nx = ey*dir[2] - ez*dir[1];
          let ny = ez*dir[0] - ex*dir[2];
          let nz = ex*dir[1] - ey*dir[0];
          const ln = Math.hypot(nx, ny, nz) || 1;
          nx /= ln; ny /= ln; nz /= ln;

          const sideId = 'face_side_' + i;
          const startTri = indices.length / 3;
          pushTri(a, b, c, nx, ny, nz, sideId);
          pushTri(a, c, d, nx, ny, nz, sideId);
          sideFaces.push({
            id: sideId, kind: 'side',
            triangleStart: startTri, triangleCount: 2,
            normal: [nx, ny, nz],
            edgeIndex: i,
          });
        }

        // 8. Build CAD-like topology
        const vertices = [];
        for (let i = 0; i < N; i++) {
          const b = baseAt(i), tp = topAt(i);
          vertices.push({ id: 'v_b_'+i, x:b.x, y:b.y, z:b.z });
          vertices.push({ id: 'v_t_'+i, x:tp.x, y:tp.y, z:tp.z });
        }
        const edges = [];
        for (let i = 0; i < N; i++) {
          const j = (i + 1) % N;
          edges.push({ id: 'e_b_'+i, a: 'v_b_'+i, b: 'v_b_'+j }); // base ring
          edges.push({ id: 'e_t_'+i, a: 'v_t_'+i, b: 'v_t_'+j }); // top ring
          edges.push({ id: 'e_s_'+i, a: 'v_b_'+i, b: 'v_t_'+i }); // side
        }
        const faces = [
          { id: baseFaceId, kind:'cap_base', triangleStart: baseTriStart, triangleCount: baseTriCount, normal: baseN },
          { id: topFaceId,  kind:'cap_top',  triangleStart: topTriStart,  triangleCount: topTriCount,  normal: dir.slice() },
          ...sideFaces,
        ];

        return {
          positions: new Float32Array(positions),
          normals:   new Float32Array(normals),
          indices:   new Uint32Array(indices),
          triangleFaceIds: triFaceIds,
          topology: { faces, edges, vertices },
          meta: {
            profileCount: N,
            depth,
            plane,
            direction: dir.slice(),
            triangleCount: indices.length / 3,
          },
        };
      };

      // ─── Solids & History stores (backend-ready) ───────────────────────────
      // Replace these with persistent backend records later.
      window.solids = window.solids || [];
      window.historyEntries = window.historyEntries || [];
      window.__nextSolidId = window.__nextSolidId || 1;

      // Reset multi-solid GPU buffer state (use on "New Project").
      window.__resetCadBuffers = function() {
        window.cadBufferState = { vertexOffset: 0, indexOffset: 0, faceIdBase: 0 };
        if (typeof cadIndexCount !== 'undefined') cadIndexCount = 0;
        window.solids = [];
        window.__nextSolidId = 1;
        if (typeof window.__renderSolidsList === 'function') {
          try { window.__renderSolidsList(); } catch(e) {}
        }
      };

      window.addHistoryEntry = function(label, payload) {
        const entry = {
          id: 'h_' + Date.now() + '_' + Math.floor(Math.random()*999),
          ts: Date.now(),
          label: label || 'Operation',
          payload: payload || null,
        };
        window.historyEntries.push(entry);
        if (typeof window.__renderHistoryList === 'function') {
          try { window.__renderHistoryList(); } catch(e) {}
        }
        return entry;
      };

      // ─── Upload generated mesh to GPU CAD buffers ──────────────────────────
      // Lives here (not in matter_ui) because it's part of the extrude pipeline.
      // Returns the solid record that was registered.
      window.uploadMeshToCadBuffers = function(mesh) {
        if (!mesh || !mesh.positions || !mesh.indices) {
          throw new Error('uploadMeshToCadBuffers: invalid mesh');
        }
        if (typeof device === 'undefined' || !device || !device.queue) {
          throw new Error('uploadMeshToCadBuffers: device not ready');
        }
        // ── Multi-solid append state ──────────────────────────────────
        window.cadBufferState = window.cadBufferState || {
          vertexOffset: 0,  // in vertices (one position = 3 floats)
          indexOffset:  0,  // in indices (one triangle = 3 indices)
          faceIdBase:   0,  // next free global face id
        };
        const st = window.cadBufferState;

        const newVerts = mesh.positions.length / 3;
        const newTris  = mesh.indices.length / 3;

        // Capacity guard (buffers were allocated for 100k verts / 100k tris)
        if (st.vertexOffset + newVerts > 100000 || st.indexOffset + mesh.indices.length > 100000 * 3) {
          throw new Error('CAD buffer full — multi-solid capacity reached (100k verts)');
        }

        // Shift indices to absolute positions inside the big buffer
        const absIndices = new Uint32Array(mesh.indices.length);
        for (let i = 0; i < mesh.indices.length; i++) {
          absIndices[i] = mesh.indices[i] + st.vertexOffset;
        }

        // Per-vertex face id (one ID per triangle expanded to 3 verts of THAT triangle).
        // Face IDs are globally unique across solids (offset by faceIdBase).
        const localFaceMap = new Map(); // name → local id
        let nextLocal = 0;
        const faceIdArr = new Uint32Array(newVerts);
        const triGlobalFaceIds = new Uint32Array(newTris); // per-triangle global id (for picking)
        for (let t = 0; t < newTris; t++) {
          const name = mesh.triangleFaceIds[t] || ('face_' + t);
          let lid = localFaceMap.get(name);
          if (lid === undefined) { lid = nextLocal++; localFaceMap.set(name, lid); }
          const globalId = st.faceIdBase + lid + 1; // 0 reserved as "none"
          faceIdArr[mesh.indices[t*3+0]] = globalId;
          faceIdArr[mesh.indices[t*3+1]] = globalId;
          faceIdArr[mesh.indices[t*3+2]] = globalId;
          triGlobalFaceIds[t] = globalId;
        }
        const faceCount = nextLocal;
        // Attach picker-friendly data back onto the mesh
        mesh.triangleGlobalFaceIds = triGlobalFaceIds;

        // Byte offsets for writeBuffer
        const posOffsetBytes    = st.vertexOffset * 3 * 4;
        const normOffsetBytes   = st.vertexOffset * 3 * 4;
        const faceIdOffsetBytes = st.vertexOffset * 4;
        const idxOffsetBytes    = st.indexOffset * 4;

        device.queue.writeBuffer(cadPosBuf,    posOffsetBytes,    mesh.positions);
        device.queue.writeBuffer(cadNormalBuf, normOffsetBytes,   mesh.normals);
        device.queue.writeBuffer(cadFaceIdBuf, faceIdOffsetBytes, faceIdArr);
        device.queue.writeBuffer(cadIndexBuf,  idxOffsetBytes,    absIndices);

        const record = {
          vertexBase:  st.vertexOffset,
          vertexCount: newVerts,
          indexBase:   st.indexOffset,
          indexCount:  mesh.indices.length,
          faceIdBase:  st.faceIdBase + 1, // global ids start at base+1
          faceCount,
        };
        st.vertexOffset += newVerts;
        st.indexOffset  += mesh.indices.length;
        st.faceIdBase   += faceCount;

        // Total drawIndexed count = end of append region
        cadIndexCount = st.indexOffset;

        return record;
      };

      // ─── Create solid: full pipeline ───────────────────────────────────────
      // sketch -> mesh -> GPU upload -> solids[] -> history -> outliner
      window.createSolidFromActiveSketch = function(opts) {
        opts = opts || {};
        const sk = (typeof sketchState !== 'undefined') ? sketchState : null;
        const depth = (window.extrudePreview && window.extrudePreview.distance) || opts.depth || 1.0;
        const dir = (window.extrudePreview && window.extrudePreview.direction) || opts.direction || null;
        const mesh = window.generateExtrudedMeshFromSketch(sk, depth, dir ? { direction: dir } : {});
        const upload = window.uploadMeshToCadBuffers(mesh);

        const id = 'solid_' + (window.__nextSolidId++);
        const solid = {
          id,
          name: 'Extrude ' + id.replace('solid_', '#'),
          kind: 'extrude',
          plane: sk.plane,
          depth,
          direction: mesh.meta.direction,
          profileCount: mesh.meta.profileCount,
          triangleCount: mesh.meta.triangleCount,
          mesh,                // keep CPU copy for future re-uploads / backend
          bufferRecord: upload,        // { vertexBase, vertexCount, indexBase, indexCount, faceIdBase, faceCount }
          faceIdBase: upload.faceIdBase,
          faceCount:  upload.faceCount,
          createdAt: Date.now(),
        };
        window.solids.push(solid);
        window.addHistoryEntry('Create Solid', { solidId: id, kind:'extrude', depth, plane: sk.plane });
        if (typeof window.__renderSolidsList === 'function') {
          try { window.__renderSolidsList(); } catch(e) {}
        }
        return solid;
      };

      // ─── Backend (truck-modeling) extrude ──────────────────────────────────
      // POST /api/matter/sketch/extrude → real B-Rep solid (vs frontend earcut).
      // Response: { positions, normals, indices, face_ids (per tri), face_count, obj_data, kernel }
      // We adapt it to the same `mesh` shape that uploadMeshToCadBuffers consumes.
      window.fetchExtrudeFromBackend = async function(sk, depth, opts) {
        opts = opts || {};
        sk = sk || (typeof sketchState !== 'undefined' ? sketchState : null);
        if (!sk) throw new Error('fetchExtrudeFromBackend: no sketchState');
        const profile3D = window.getActiveProfilePoints(sk);
        if (!profile3D || profile3D.length < 3) {
          throw new Error('fetchExtrudeFromBackend: no closed profile');
        }
        const plane = sk.plane || 'XZ';
        const ax = getPlaneAxes(plane);
        const dir = opts.direction || ax.n;
        const body = {
          plane,
          depth: depth > 0 ? depth : 1.0,
          direction: [dir[0], dir[1], dir[2]],
          profile: profile3D.map(p => ({
            x: typeof p.x === 'number' ? p.x : (p[0] || 0),
            y: typeof p.y === 'number' ? p.y : (p[1] || 0),
            z: typeof p.z === 'number' ? p.z : (p[2] || 0),
          })),
          tolerance: opts.tolerance || 0.01,
        };
        const resp = await fetch('/api/matter/sketch/extrude', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        });
        if (!resp.ok) {
          const txt = await resp.text();
          throw new Error('backend ' + resp.status + ': ' + txt);
        }
        const data = await resp.json();
        // Adapt to uploadMeshToCadBuffers contract:
        //   triangleFaceIds: string[]  ← convert numeric face_ids to "face_<n>"
        const triFaceIds = new Array(data.triangle_count);
        for (let i = 0; i < data.triangle_count; i++) {
          triFaceIds[i] = 'face_' + data.face_ids[i];
        }
        return {
          positions: new Float32Array(data.positions),
          normals:   new Float32Array(data.normals),
          indices:   new Uint32Array(data.indices),
          triangleFaceIds: triFaceIds,
          topology: { faces: [], edges: [], vertices: [] }, // backend B-Rep topology TBD
          meta: {
            profileCount: profile3D.length,
            depth: body.depth,
            plane,
            direction: body.direction,
            triangleCount: data.triangle_count,
            kernel: data.kernel || 'truck-modeling',
            faceCount: data.face_count,
            objData: data.obj_data,
          },
        };
      };

      // Async full pipeline: backend → upload → solid record (with local fallback).
      window.createSolidFromActiveSketchAsync = async function(opts) {
        opts = opts || {};
        const sk = (typeof sketchState !== 'undefined') ? sketchState : null;
        const depth = (window.extrudePreview && window.extrudePreview.distance) || opts.depth || 1.0;
        const dir = (window.extrudePreview && window.extrudePreview.direction) || opts.direction || null;

        let mesh, source;
        try {
          mesh = await window.fetchExtrudeFromBackend(sk, depth, dir ? { direction: dir } : {});
          source = 'truck-modeling';
        } catch (err) {
          console.warn('[extrude] backend failed, falling back to local earcut:', err.message);
          mesh = window.generateExtrudedMeshFromSketch(sk, depth, dir ? { direction: dir } : {});
          source = 'local-earcut';
        }
        const upload = window.uploadMeshToCadBuffers(mesh);

        const id = 'solid_' + (window.__nextSolidId++);
        const solid = {
          id,
          name: 'Extrude ' + id.replace('solid_', '#'),
          kind: 'extrude',
          plane: sk.plane,
          depth,
          direction: mesh.meta.direction,
          profileCount: mesh.meta.profileCount,
          triangleCount: mesh.meta.triangleCount,
          mesh,
          source,
          bufferRecord: upload,
          faceIdBase: upload.faceIdBase,
          faceCount:  upload.faceCount,
          createdAt: Date.now(),
        };
        window.solids.push(solid);
        window.addHistoryEntry('Create Solid', { solidId: id, kind: 'extrude', depth, plane: sk.plane, source });
        if (typeof window.__renderSolidsList === 'function') {
          try { window.__renderSolidsList(); } catch(e) {}
        }
        return solid;
      };
"##;
