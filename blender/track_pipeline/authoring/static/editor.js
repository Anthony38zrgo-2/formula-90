/* F90 Track Authoring editor — Vue 3 + SVG.js (local vendor bundles).
   The SVG document is canonical authority; the server sanitizes every save.
   After each save/undo/redo the canonical response is re-rendered, so stable
   per-instance IDs and validated geometry are never re-derived from DOM order.
   Visual acceptance requires a human/image-capable reviewer. */

const { createApp, reactive, computed } = Vue;

const SVG_NS = "http://www.w3.org/2000/svg";
const F90_NS = "urn:formula90:track";
const EDITOR_ONLY = "data-editor-only";

const api = {
  async getTrack() { return (await fetch("/api/track")).json(); },
  async getAssets() { return (await fetch("/api/assets")).json(); },
  async save(svg) {
    return (await fetch("/api/track", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ svg }),
    })).json();
  },
  async undo() { return (await fetch("/api/undo", { method: "POST" })).json(); },
  async redo() { return (await fetch("/api/redo", { method: "POST" })).json(); },
  async region(operation, region_id, params) {
    return (await fetch("/api/region", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ operation, region_id, params }),
    })).json();
  },
  async validate() { return (await fetch("/api/validate")).json(); },
  async importSvg(svg) {
    const res = await fetch("/api/import", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ svg }),
    });
    const body = await res.json();
    body.http_status = res.status;
    return body;
  },
};

let draw = null;           // SVG.js drawing
let baseRoot = null;       // last parsed SVG root (viewBox + data attrs)
let drag = null;           // active drag state
let autoSaveTimer = null;

const app = createApp({
  setup() {
    const state = reactive({
      selection: null,
      undoDepth: 0,
      redoDepth: 0,
      trackId: "…",
      validation: null,
      assets: [],
      roadWidth: 12,
      surfaceElevation: 0.025,
      centerline: null,
      banking: [],
      elevation: [],
      terrain: [],
      barriers: [],
      markers: [],
      regions: [],
      categoryVisibility: { grass: true, bushes: true, trees: true, cards: true },
      regionForm: { dx: 0, dz: 0, factor: 1, boundaryText: "" },
      regionError: null,
      pickRegionMode: false,
      pendingPlace: null,
      importText: "",
      importResult: null,
      canvasEl: null,
    });

    const validationText = computed(() => {
      if (!state.validation) return "not validated";
      return state.validation.ok ? "VALID" : "INVALID";
    });
    const validationOk = computed(() =>
      state.validation ? state.validation.ok : null);

    const selectionRole = computed(() =>
      state.selection ? state.selection.getAttribute("data-role") : null);
    const selNode = computed(() => state.selection);

    const selPosX = computed({
      get() { return num(selNode.value, "cx", 0); },
      set(v) { setNum(selNode.value, "cx", v); },
    });
    const selPosZ = computed({
      get() { return num(selNode.value, "cy", 0); },
      set(v) { setNum(selNode.value, "cy", v); },
    });
    const selScale = computed({
      get() { return num(selNode.value, "data-scale", 1); },
      set(v) { setNum(selNode.value, "data-scale", v); },
    });
    const selYaw = computed({
      get() { return num(selNode.value, "data-yaw-rad", 0); },
      set(v) { setNum(selNode.value, "data-yaw-rad", v); },
    });

    const selBanking = computed(() => findModel(state.banking, state.selection));
    const selElevation = computed(() => findModel(state.elevation, state.selection));
    const selBarrier = computed(() => findModel(state.barriers, state.selection));
    const selTerrain = computed(() => findModel(state.terrain, state.selection));
    const selMarker = computed(() => findModel(state.markers, state.selection));
    const importResult = computed(() => state.importResult);
    const trackId = computed(() => state.trackId);
    const undoDepth = computed(() => state.undoDepth);
    const redoDepth = computed(() => state.redoDepth);

    const selRegion = computed(() => {
      const sel = state.selection;
      if (!sel) return null;
      return state.regions.find((r) => r.node === sel) || null;
    });
    const vegCategories = ["grass", "bushes", "trees"];

    return { state, validationText, validationOk, selectionRole, selNode,
             selPosX, selPosZ, selScale, selYaw,
             selBanking, selElevation, selBarrier, selTerrain, selMarker,
             importResult, trackId, undoDepth, redoDepth,
             selRegion, vegCategories };
  },

  methods: {
    async load() {
      const [track, assets] = await Promise.all([api.getTrack(), api.getAssets()]);
      this.state.trackId = track.track_id || "…";
      this.state.undoDepth = track.undo_depth;
      this.state.redoDepth = track.redo_depth;
      this.state.assets = (assets.ok && assets.assets) || [];
      renderSvg(this, track.current);
      this.clearSelection();
    },

    select(node) {
      this.state.selection = node;
      clearSelectionClasses();
      if (node) node.classList.add("selected");
      renderHandles(this);
    },

    clearSelection() {
      this.state.selection = null;
      this.state.pendingPlace = null;
      clearSelectionClasses();
      renderHandles(this);
    },

    selIs(node) {
      return this.state.selection === node;
    },

    selectControl(type, index) {
      const item = this.state[type] && this.state[type][index];
      if (item) this.select(item.node);
    },

    // -- placement --------------------------------------------------------

    placeFromPalette(spec) {
      this.state.pendingPlace = { kind: "asset", spec };
    },

    placeAt(pt) {
      const p = this.state.pendingPlace;
      if (!p) return;
      let el = null;
      if (p.kind === "asset") {
        el = document.createElementNS(SVG_NS, "circle");
        el.setAttribute("data-role", "asset-instance");
        el.setAttribute("data-asset-id", p.spec.id);
        el.setAttribute("data-kind", p.spec.kind);
        el.setAttribute("data-scale", "1");
        el.setAttribute("data-yaw-rad", "0");
        el.setAttribute("r", "0.4");
      } else if (p.kind === "marker") {
        el = document.createElementNS(SVG_NS, "circle");
        el.setAttribute("data-role", "marker");
        el.setAttribute("data-kind", "start");
        el.setAttribute("r", "0.6");
      } else if (p.kind === "barrier") {
        el = document.createElementNS(SVG_NS, "line");
        el.setAttribute("data-role", "barrier");
        el.setAttribute("data-kind", "guardrail");
        el.setAttribute("x1", String(pt.x - 3));
        el.setAttribute("y1", String(pt.y));
        el.setAttribute("x2", String(pt.x + 3));
        el.setAttribute("y2", String(pt.y));
      } else if (p.kind === "terrain") {
        el = document.createElementNS(SVG_NS, "polygon");
        el.setAttribute("data-role", "terrain-zone");
        el.setAttribute("data-kind", "grass");
        el.setAttribute("data-name", "zone");
        const r = 8;
        el.setAttribute("points",
          `${pt.x - r},${pt.y - r} ${pt.x + r},${pt.y - r} ${pt.x + r},${pt.y + r} ${pt.x - r},${pt.y + r}`);
      }
      if (!el) return;
      if (p.kind !== "barrier" && p.kind !== "terrain") {
        el.setAttribute("cx", String(pt.x));
        el.setAttribute("cy", String(pt.y));
      }
      const role = el.getAttribute("data-role");
      const cls = roleClass(role);
      if (cls) el.classList.add(cls);
      layerGroupFor(role).appendChild(el);
      this.state.pendingPlace = null;
      bindElementEvents(el);
      this.select(el);
      this.populateModel();
      this.commit();
    },

    // -- control add / remove ---------------------------------------------

    addControl(type) {
      const el = document.createElementNS(SVG_NS, "path");
      el.setAttribute("data-role", type);
      if (type === "banking") {
        el.setAttribute("data-s-m", "0");
        el.setAttribute("data-degrees", "0");
      } else if (type === "elevation") {
        el.setAttribute("data-s-m", "0");
        el.setAttribute("data-height-m", "0");
      }
      el.classList.add(roleClass(type));
      layerGroupFor(type).appendChild(el);
      bindElementEvents(el);
      this.select(el);
      this.populateModel();
      this.commit();
    },

    delSelectedControl(modelKey) {
      const item = findModel(this.state[modelKey], this.state.selection);
      if (!item) return;
      item.node.remove();
      this.clearSelection();
      this.populateModel();
      this.commit();
    },

    applyControl(item, pairs) {
      if (!item || !item.node) return;
      for (const [attr, value] of Object.entries(pairs)) {
        const v = typeof value === "number" ? String(Number(value)) : String(value);
        item.node.setAttribute(attr, v);
      }
      this.commit();
    },

    // -- terrain points ----------------------------------------------------

    applyTerrainPoint(t, i) {
      if (!t || !t.node) return;
      t.node.setAttribute("points", pointsAttr(t.points));
      this.commit();
    },

    addTerrainPoint(t) {
      if (!t) return;
      const a = t.points[0] || [0, 0];
      const b = t.points[t.points.length - 1] || a;
      t.points.push([(a[0] + b[0]) / 2, (a[1] + b[1]) / 2]);
      t.node.setAttribute("points", pointsAttr(t.points));
      this.commit();
    },

    delTerrainPoint(t, i) {
      if (!t || t.points.length <= 3) return;
      t.points.splice(i, 1);
      t.node.setAttribute("points", pointsAttr(t.points));
      this.commit();
    },

    // -- centerline ---------------------------------------------------------

    applyCenterlinePoint(i) {
      const c = this.state.centerline;
      if (!c) return;
      c.node.setAttribute("d", pointsToPath(c.points));
      this.commit();
    },

    addCenterlinePoint() {
      const c = this.state.centerline;
      if (!c || c.points.length < 2) return;
      const a = c.points[0];
      const b = c.points[c.points.length - 1];
      c.points.push([(a[0] + b[0]) / 2, (a[1] + b[1]) / 2]);
      c.node.setAttribute("d", pointsToPath(c.points));
      renderHandles(this);
      this.commit();
    },

    delCenterlinePoint(i) {
      const c = this.state.centerline;
      if (!c || c.points.length <= 3) return;
      c.points.splice(i, 1);
      c.node.setAttribute("d", pointsToPath(c.points));
      renderHandles(this);
      this.commit();
    },

    // -- track root attributes ----------------------------------------------

    writeRoad(attr) {
      const value = attr === "data-road-width-m" ? this.state.roadWidth : this.state.surfaceElevation;
      draw.node.setAttribute(attr, String(Number(value)));
      this.commit();
    },

    // -- commit / save ------------------------------------------------------

    commitSelection() {
      this.commit();
    },

    commit() {
      clearTimeout(autoSaveTimer);
      autoSaveTimer = setTimeout(() => this.save(), 200);
    },

    async save() {
      const payload = serializeDoc(this);
      const s = await api.save(payload);
      this.afterState(s);
    },

    afterState(s) {
      this.state.undoDepth = s.undo_depth;
      this.state.redoDepth = s.redo_depth;
      this.state.trackId = s.track_id || this.state.trackId;
      if (s.current) renderSvg(this, s.current);
      this.state.validation = null;
      this.clearSelection();
    },

    async undo() {
      const s = await api.undo();
      this.afterState(s);
    },

    async redo() {
      const s = await api.redo();
      this.afterState(s);
    },

    async validate() {
      const v = await api.validate();
      this.state.validation = v;
      this.populateModel();
    },

    // -- import --------------------------------------------------------------

    async doImport() {
      if (!this.state.importText.trim()) return;
      const res = await api.importSvg(this.state.importText);
      this.state.importResult = res;
      if (res.stage === "imported" && res.validation && res.validation.track_id) {
        this.state.trackId = res.validation.track_id;
      }
    },

    async useImport() {
      const res = this.state.importResult;
      if (!res || res.stage !== "imported") return;
      const s = await api.save(res.canonical);
      this.state.importResult = null;
      this.state.importText = "";
      this.afterState(s);
    },

    // -- asset duplication ---------------------------------------------------

    async duplicateAsset() {
      const node = this.state.selection;
      if (!node || node.getAttribute("data-role") !== "asset-instance") return;
      const clone = node.cloneNode(true);
      clone.removeAttribute("data-instance-id");
      clone.setAttribute("cx", String(num(node, "cx") + 1.5));
      clone.setAttribute("cy", String(num(node, "cy") + 1.5));
      layerGroupFor("asset-instance").appendChild(clone);
      bindElementEvents(clone);
      this.select(clone);
      this.populateModel();
      this.commit();
    },

    async deleteAsset() {
      const node = this.state.selection;
      if (!node || node.getAttribute("data-role") !== "asset-instance") return;
      node.remove();
      this.clearSelection();
      this.populateModel();
      this.commit();
    },

    // -- model rebuild --------------------------------------------------------

    populateModel() {
      const root = draw.node;
      this.state.roadWidth = num(root, "data-road-width-m", 12);
      this.state.surfaceElevation = num(root, "data-road-surface-elevation-m", 0.025);
      const cl = q("[data-role='centerline']");
      this.state.centerline = cl
        ? { node: cl, points: parsePathPoints(cl.getAttribute("d") || "") }
        : null;
      this.state.banking = collectRole("banking", (el) => ({
        node: el, s: num(el, "data-s-m", 0), degrees: num(el, "data-degrees", 0),
      }));
      this.state.elevation = collectRole("elevation", (el) => ({
        node: el, s: num(el, "data-s-m", 0), height: num(el, "data-height-m", 0),
      }));
      this.state.barriers = collectRole("barrier", (el) => ({
        node: el, kind: el.getAttribute("data-kind") || "guardrail",
        x1: num(el, "x1", 0), y1: num(el, "y1", 0),
        x2: num(el, "x2", 0), y2: num(el, "y2", 0),
      }));
      this.state.terrain = collectRole("terrain-zone", (el) => ({
        node: el, name: el.getAttribute("data-name") || "zone",
        kind: el.getAttribute("data-kind") || "grass",
        points: parsePoints(el.getAttribute("points") || ""),
      }));
      this.state.markers = collectRole("marker", (el) => ({
        node: el, kind: el.getAttribute("data-kind") || "start",
        x: num(el, "cx", 0), z: num(el, "cy", 0),
      }));
      this.populateRegions();
      this.applyCategoryVisibility();
    },

    populateRegions() {
      this.state.regions = collectRole("vegetation-region", (g) => ({
        node: g,
        id: g.getAttribute("data-region-id"),
        category: g.getAttribute("data-category") || "vegetation",
        seed: num(g, "data-seed", 0),
        spacing: num(g, "data-spacing-m", 4),
        targetCount: num(g, "data-target-count", 0),
        memberCount: [...g.children]
          .filter((c) => c.getAttribute("data-role") === "asset-instance").length,
        boundary: boundaryOf(g),
      }));
    },

    // -- vegetation regions --------------------------------------------------

    regionsByCategory(cat) {
      return this.state.regions.filter((r) => r.category === cat);
    },

    selectRegion(id) {
      const r = this.state.regions.find((x) => x.id === id) || null;
      const el = r ? r.node : null;
      if (!el) return r;
      this.select(el);
      this.state.regionForm.boundaryText = r.boundary
        .map((p) => `${fmtN(p[0])},${fmtN(p[1])}`).join("\n");
      this.state.pickRegionMode = false;
      return r;
    },

    pickNearestRegion(pt) {
      let best = null;
      let bestD = Infinity;
      for (const r of this.state.regions) {
        for (const p of r.boundary) {
          const d = Math.hypot(p[0] - pt.x, p[1] - pt.y);
          if (d < bestD) { bestD = d; best = r; }
        }
      }
      if (best) this.selectRegion(best.id);
      this.state.pickRegionMode = false;
    },

    applyCategoryVisibility() {
      const keys = this.state.categoryVisibility;
      qAll("[data-role='vegetation-region']").forEach((g) => {
        const cat = g.getAttribute("data-category");
        g.classList.toggle("hidden", keys[cat] === false);
      });
      qAll("[data-role='asset-instance']").forEach((el) => {
        if (el.getAttribute("data-generated-by-region")) return;
        const key = visibilityKeyOf(el);
        if (!key) return;
        el.classList.toggle("hidden", keys[key] === false);
      });
    },

    applyRegionSpacing() {
      const r = this.selRegion;
      if (!r) return;
      r.node.setAttribute("data-spacing-m", String(Number(r.spacing)));
      this.commit();
    },

    assetPoolFor(r) {
      const pool = this.state.assets
        .filter((a) => a.category === r.category).map((a) => a.id);
      if (pool.length) return pool;
      return this.state.assets
        .filter((a) => a.kind === "vegetation").map((a) => a.id);
    },

    regionResult(res, regionId) {
      if (res.ok) {
        this.state.regionError = null;
        this.afterState(res);
        this.selectRegion(regionId);
      } else {
        this.state.regionError = res.diagnostics || [res.error || "region operation failed"];
      }
    },

    async applyRegionMove() {
      const r = this.selRegion;
      if (!r) return;
      const res = await api.region("move", r.id, {
        dx: this.state.regionForm.dx,
        dz: this.state.regionForm.dz,
      });
      this.regionResult(res, r.id);
    },

    async applyRegionScale() {
      const r = this.selRegion;
      if (!r) return;
      const res = await api.region("scale", r.id, {
        factor: this.state.regionForm.factor,
      });
      this.regionResult(res, r.id);
    },

    async applyRegionExtend() {
      const r = this.selRegion;
      if (!r) return;
      const pts = parsePoints(this.state.regionForm.boundaryText);
      if (pts.length < 3) {
        this.state.regionError = ["new boundary needs at least 3 points (x,z per line)"];
        return;
      }
      const res = await api.region("extend", r.id, {
        new_boundary: pts,
        asset_pool: this.assetPoolFor(r),
      });
      this.regionResult(res, r.id);
    },
  },

  mounted() {
    draw = SVG().addTo(this.$refs.canvas);
    this.state.canvasEl = draw.node;
    draw.node.addEventListener("pointerdown", (ev) => {
      if (ev.target !== draw.node) return;
      if (this.state.pendingPlace) {
        const pt = toUserPoint(ev);
        this.placeAt(pt);
      }
    });
    draw.node.addEventListener("pointerdown", (ev) => {
      if (!this.state.pickRegionMode) return;
      ev.preventDefault();
      ev.stopPropagation();
      this.pickNearestRegion(toUserPoint(ev));
    }, true);
    window.addEventListener("keydown", (ev) => {
      if (ev.key === "Escape") {
        this.state.pendingPlace = null;
        this.state.pickRegionMode = false;
      }
    });
    bindDrag(this);
    this.load();
  },
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function num(node, attr, dflt = 0) {
  if (!node) return dflt;
  const v = node.getAttribute(attr);
  return v == null ? dflt : parseFloat(v);
}
function setNum(node, attr, v) {
  if (!node) return;
  node.setAttribute(attr, String(Number(v)));
}
function q(sel) { return draw.node.querySelector(sel); }
function qAll(sel) { return [...draw.node.querySelectorAll(sel)]; }

function roleClass(role) {
  return { centerline: "centerline", road: "road", banking: "banking",
           "terrain-zone": "terrain", barrier: "barrier",
           "asset-instance": "asset", elevation: "elevation", marker: "marker" }[role];
}

function layerGroupFor(role) {
  const layer = role === "asset-instance" ? "assets"
    : role === "barrier" ? "barrier"
    : role === "terrain-zone" ? "terrain"
    : role === "marker" ? "markers" : "road";
  let g = q(`g[data-layer='${layer}']`);
  if (!g) {
    g = document.createElementNS(SVG_NS, "g");
    g.setAttribute("data-layer", layer);
    draw.node.appendChild(g);
  }
  return g;
}

function findModel(items, node) {
  if (!items) return null;
  return items.find((item) => item.node === node) || null;
}

function collectRole(role, mapper) {
  return qAll(`[data-role='${role}']`).map(mapper);
}

function boundaryOf(g) {
  const poly = [...g.children].find(
    (c) => c.getAttribute("data-role") === "vegetation-boundary");
  return poly ? parsePoints(poly.getAttribute("points") || "") : [];
}

function visibilityKeyOf(el) {
  const cat = el.getAttribute("data-category");
  if (cat && ["grass", "bushes", "trees"].includes(cat)) return cat;
  const kind = el.getAttribute("data-kind") || "";
  if (kind === "tree") return "trees";
  if (kind === "card" || kind === "flag" || kind === "person") return "cards";
  return null;
}

function clearSelectionClasses() {
  qAll("[data-role]").forEach((n) => n.classList.remove("selected"));
}

function bindElementEvents(node) {
  node.addEventListener("pointerdown", (ev) => {
    ev.preventDefault();
    const vm = app._instance.proxy;
    const role = node.getAttribute("data-role");
    if (role === "asset-instance") {
      const gen = node.getAttribute("data-generated-by-region");
      if (gen) {
        vm.selectRegion(gen);
        return;
      }
      vm.select(node);
      bindAssetDrag(ev, node);
    } else if (role === "vegetation-region") {
      vm.selectRegion(node.getAttribute("data-region-id"));
    } else if (role === "vegetation-boundary") {
      const region = node.parentNode;
      if (region && region.getAttribute("data-role") === "vegetation-region") {
        vm.selectRegion(region.getAttribute("data-region-id"));
      } else {
        vm.select(node);
      }
    } else {
      vm.select(node);
    }
  });
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

function renderSvg(vm, svgText) {
  const parser = new DOMParser();
  const doc = parser.parseFromString(svgText, "image/svg+xml");
  baseRoot = doc.documentElement;
  const viewBox = baseRoot.getAttribute("viewBox") || "0 0 100 60";
  const vb = viewBox.replace(/,/g, " ").split(" ").map(Number);
  draw.clear();
  draw.viewbox(vb[0], vb[1], vb[2], vb[3]);
  draw.svg(baseRoot.innerHTML);
  const root = draw.node;
  root.setAttribute("viewBox", viewBox);
  for (const a of baseRoot.attributes) {
    if (a.name.startsWith("data-")) root.setAttribute(a.name, a.value);
  }
  qAll("[data-role]").forEach((el) => {
    const cls = roleClass(el.getAttribute("data-role"));
    if (cls) el.classList.add(cls);
    bindElementEvents(el);
  });
  vm.populateModel();
  renderHandles(vm);
}

function renderHandles(vm) {
  removeHandles();
  const sel = vm.state.selection;
  if (!sel) return;
  const role = sel.getAttribute("data-role");
  let points = null;
  let kind = null;
  if (role === "centerline") {
    points = parsePathPoints(sel.getAttribute("d") || "");
    kind = "cl-point";
  } else if (role === "barrier") {
    points = [[num(sel, "x1", 0), num(sel, "y1", 0)], [num(sel, "x2", 0), num(sel, "y2", 0)]];
    kind = "barrier-end";
  }
  if (!points || !kind) return;
  const g = document.createElementNS(SVG_NS, "g");
  g.setAttribute(EDITOR_ONLY, "true");
  points.forEach((p, i) => {
    const c = document.createElementNS(SVG_NS, "circle");
    c.setAttribute("cx", String(p[0]));
    c.setAttribute("cy", String(p[1]));
    c.setAttribute("r", "1.6");
    c.setAttribute("class", "node");
    c.setAttribute("data-point-index", String(i));
    g.appendChild(c);
    c.addEventListener("pointerdown", (ev) => {
      ev.preventDefault();
      ev.stopPropagation();
      const vm2 = app._instance.proxy;
      vm2.select(sel);
      const pt = toUserPoint(ev);
      if (kind === "cl-point") {
        drag = {
          kind, node: sel, g, index: i, start: pt, origin: [p[0], p[1]],
          points: parsePathPoints(sel.getAttribute("d") || ""),
        };
      } else {
        drag = { kind, node: sel, g, index: i, start: pt, origin: [p[0], p[1]] };
      }
    });
  });
  draw.node.appendChild(g);
}

function removeHandles() {
  const g = q(`g[${EDITOR_ONLY}='true']`);
  if (g) g.remove();
}

// ---------------------------------------------------------------------------
// Path helpers (deterministic subset of the profile grammar)
// ---------------------------------------------------------------------------

function _tokenizePath(d) {
  const tokens = [];
  let number = "";
  for (const ch of (d || "").replace(/,/g, " ")) {
    if ("MmLlHhVvCcQqZz".includes(ch)) {
      if (number.trim()) {
        tokens.push(...number.trim().split(/\s+/));
        number = "";
      }
      tokens.push(ch);
    } else {
      number += ch;
    }
  }
  if (number.trim()) tokens.push(...number.trim().split(/\s+/));
  return tokens;
}

function flattenCubic(x0, y0, x1, y1, x2, y2, x3, y3, tol, out) {
  const flat = Math.hypot(x2 - (3 * x1 - 2 * x0), y2 - (3 * y1 - 2 * y0)) <= tol
    && Math.hypot(x1 - x0, y1 - y0) <= tol;
  if (flat) {
    out.push([x3, y3]);
    return;
  }
  const ax = (x0 + x1) / 2, ay = (y0 + y1) / 2;
  const bx = (x1 + x2) / 2, by = (y1 + y2) / 2;
  const cx2 = (x2 + x3) / 2, cy2 = (y2 + y3) / 2;
  const abx = (ax + bx) / 2, aby = (ay + by) / 2;
  const bcx = (bx + cx2) / 2, bcy = (by + cy2) / 2;
  const mx = (abx + bcx) / 2, my = (aby + bcy) / 2;
  flattenCubic(x0, y0, ax, ay, abx, aby, mx, my, tol, out);
  flattenCubic(mx, my, bcx, bcy, cx2, cy2, x3, y3, tol, out);
}

function parsePathPoints(d) {
  const tokens = _tokenizePath(d);
  const pts = [];
  let cx = 0, cy = 0, start = [0, 0];
  let i = 0;
  while (i < tokens.length) {
    const cmd = tokens[i][0];
    i += 1;
    const params = [];
    while (i < tokens.length && !"MmLlHhVvCcQqZz".includes(tokens[i][0])) {
      params.push(Number(tokens[i]));
      i += 1;
    }
    if (cmd === "M" || cmd === "m") {
      for (let k = 0; k + 1 < params.length; k += 2) {
        const x = cmd === "m" ? cx + params[k] : params[k];
        const y = cmd === "m" ? cy + params[k + 1] : params[k + 1];
        cx = x; cy = y; start = [x, y];
        pts.push([x, y]);
      }
    } else if (cmd === "L" || cmd === "l") {
      for (let k = 0; k + 1 < params.length; k += 2) {
        const x = cmd === "l" ? cx + params[k] : params[k];
        const y = cmd === "l" ? cy + params[k + 1] : params[k + 1];
        cx = x; cy = y;
        pts.push([x, y]);
      }
    } else if (cmd === "H" || cmd === "h") {
      cx = cmd === "h" ? cx + params[params.length - 1] : params[params.length - 1];
      pts.push([cx, cy]);
    } else if (cmd === "V" || cmd === "v") {
      cy = cmd === "v" ? cy + params[params.length - 1] : params[params.length - 1];
      pts.push([cx, cy]);
    } else if (cmd === "C" || cmd === "c") {
      const flat = [];
      let p0 = [cx, cy];
      for (let k = 0; k + 5 < params.length; k += 6) {
        let c1 = [params[k], params[k + 1]];
        let c2 = [params[k + 2], params[k + 3]];
        let e = [params[k + 4], params[k + 5]];
        if (cmd === "c") {
          c1 = [p0[0] + c1[0], p0[1] + c1[1]];
          c2 = [p0[0] + c2[0], p0[1] + c2[1]];
          e = [p0[0] + e[0], p0[1] + e[1]];
        }
        flattenCubic(p0[0], p0[1], c1[0], c1[1], c2[0], c2[1], e[0], e[1], 0.1, flat);
        p0 = e;
      }
      flat.forEach((fp) => { cx = fp[0]; cy = fp[1]; pts.push(fp); });
    } else if (cmd === "Q" || cmd === "q") {
      const flat = [];
      let p0 = [cx, cy];
      for (let k = 0; k + 3 < params.length; k += 4) {
        let qc = [params[k], params[k + 1]];
        let e = [params[k + 2], params[k + 3]];
        if (cmd === "q") {
          qc = [p0[0] + qc[0], p0[1] + qc[1]];
          e = [p0[0] + e[0], p0[1] + e[1]];
        }
        const c1 = [p0[0] + (2 / 3) * (qc[0] - p0[0]), p0[1] + (2 / 3) * (qc[1] - p0[1])];
        const c2 = [e[0] + (2 / 3) * (qc[0] - e[0]), e[1] + (2 / 3) * (qc[1] - e[1])];
        flattenCubic(p0[0], p0[1], c1[0], c1[1], c2[0], c2[1], e[0], e[1], 0.1, flat);
        p0 = e;
      }
      flat.forEach((fp) => { cx = fp[0]; cy = fp[1]; pts.push(fp); });
    } else if (cmd === "Z" || cmd === "z") {
      cx = start[0]; cy = start[1];
    }
  }
  if (pts.length > 1) {
    const a = pts[0], b = pts[pts.length - 1];
    if (a[0] === b[0] && a[1] === b[1]) pts.pop();
  }
  return pts;
}

function fmtN(v) { return String(Math.round(Number(v) * 1e4) / 1e4); }

function pointsToPath(points) {
  if (!points || !points.length) return "";
  const parts = [`M ${fmtN(points[0][0])} ${fmtN(points[0][1])}`];
  for (let i = 1; i < points.length; i += 1) {
    parts.push(`L ${fmtN(points[i][0])} ${fmtN(points[i][1])}`);
  }
  parts.push("Z");
  return parts.join(" ");
}

function parsePoints(s) {
  const nums = (s || "").replace(/,/g, " ").trim().split(/\s+/).map(Number);
  const pts = [];
  for (let i = 0; i + 1 < nums.length; i += 2) pts.push([nums[i], nums[i + 1]]);
  return pts;
}

function pointsAttr(points) {
  return points.map((p) => `${fmtN(p[0])},${fmtN(p[1])}`).join(" ");
}

// ---------------------------------------------------------------------------
// Drag + serialization
// ---------------------------------------------------------------------------

function bindAssetDrag(ev, node) {
  const vm = app._instance.proxy;
  vm.select(node);
  const pt = toUserPoint(ev);
  drag = {
    kind: "asset",
    node,
    start: pt,
    origin: { x: num(node, "cx"), z: num(node, "cy") },
  };
}

function bindDrag(vm) {
  window.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    const pt = toUserPoint(ev);
    if (drag.kind === "asset") {
      drag.node.setAttribute("cx", String(pt.x));
      drag.node.setAttribute("cy", String(pt.y));
    } else if (drag.kind === "cl-point") {
      const nx = drag.origin[0] + (pt.x - drag.start.x);
      const nz = drag.origin[1] + (pt.y - drag.start.y);
      drag.points[drag.index] = [nx, nz];
      drag.node.setAttribute("d", pointsToPath(drag.points));
      moveHandle(drag.g, drag.index, nx, nz);
      const c = app._instance.proxy.state.centerline;
      if (c) c.points[drag.index] = [nx, nz];
    } else if (drag.kind === "barrier-end") {
      const nx = drag.origin[0] + (pt.x - drag.start.x);
      const nz = drag.origin[1] + (pt.y - drag.start.y);
      if (drag.index === 0) {
        drag.node.setAttribute("x1", String(nx));
        drag.node.setAttribute("y1", String(nz));
      } else {
        drag.node.setAttribute("x2", String(nx));
        drag.node.setAttribute("y2", String(nz));
      }
      moveHandle(drag.g, drag.index, nx, nz);
      const m = app._instance.proxy.state.barriers.find((b) => b.node === drag.node);
      if (m) {
        if (drag.index === 0) { m.x1 = nx; m.y1 = nz; }
        else { m.x2 = nx; m.y2 = nz; }
      }
    }
  });
  window.addEventListener("pointerup", () => {
    if (!drag) return;
    drag = null;
    app._instance.proxy.commit();
  });
}

function moveHandle(g, index, x, z) {
  const h = g.querySelector(`[data-point-index='${index}']`);
  if (h) {
    h.setAttribute("cx", String(x));
    h.setAttribute("cy", String(z));
  }
}

function toUserPoint(ev) {
  const svg = document.querySelector("#canvas svg");
  const rect = svg.getBoundingClientRect();
  const vb = svg.viewBox.baseVal;
  const x = vb.x + (ev.clientX - rect.left) * (vb.width / rect.width);
  const y = vb.y + (ev.clientY - rect.top) * (vb.height / rect.height);
  return { x, y };
}

function esc(s) {
  return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");
}

function serializeDoc(vm) {
  const inner = draw.svg();
  const doc = new DOMParser().parseFromString(
    `<svg xmlns="${SVG_NS}" xmlns:f90="${F90_NS}">${inner}</svg>`, "image/svg+xml");
  doc.querySelectorAll(`[${EDITOR_ONLY}]`).forEach((n) => n.remove());
  const root = draw.node;
  const attrs = [];
  if (root.getAttribute("viewBox")) attrs.push(`viewBox="${esc(root.getAttribute("viewBox"))}"`);
  for (const a of root.attributes) {
    if (a.name.startsWith("data-")) attrs.push(`${a.name}="${esc(a.value)}"`);
  }
  return `<svg xmlns="${SVG_NS}" xmlns:f90="${F90_NS}" ${attrs.join(" ")}>${doc.documentElement.innerHTML}</svg>`;
}

app.mount("#app");
