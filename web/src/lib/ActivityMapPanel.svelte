<script lang="ts">
  import { onMount } from 'svelte';
  import 'leaflet/dist/leaflet.css';
  import { feature } from 'topojson-client';
  import world from 'world-atlas/countries-110m.json';
  import usStates from 'us-atlas/states-10m.json';
  import type { GeoJSON as LeafletGeoJSON, LayerGroup, Map as LeafletMap, TileLayer } from 'leaflet';
  import { api } from './api';
  import type { ActivityMapPoint, Club, Event, HostedMapConfig, ManagedCallsign } from './types';

  let { clubs, identities, events }: { clubs: Club[]; identities: ManagedCallsign[]; events: Event[] } = $props();
  let scope = $state('overall');
  let points = $state<ActivityMapPoint[]>([]);
  let loading = $state(false);
  let notice = $state('');
  let selected = $state<ActivityMapPoint | null>(null);
  let showCountries = $state(true);
  let showStates = $state(true);
  let showCounties = $state(false);
  let showLabels = $state(true);
  let showGridFields = $state(true);
  let showGridSquares = $state(false);
  let showExactGrids = $state(true);
  let showFineGrids = $state(true);
  let mapElement = $state<HTMLDivElement | null>(null);
  let leaflet: typeof import('leaflet') | null = null;
  let map: LeafletMap | null = null;
  let landLayer: LeafletGeoJSON | null = null;
  let tileLayer: TileLayer | null = null;
  let countryLayer: LeafletGeoJSON | null = null;
  let stateLayer: LeafletGeoJSON | null = null;
  let countyLayer: LeafletGeoJSON | null = null;
  let gridFieldsLayer: LayerGroup | null = null;
  let gridSquaresLayer: LayerGroup | null = null;
  let exactGridLayer: LayerGroup | null = null;
  let fineGridLayer: LayerGroup | null = null;
  let markerLayer: LayerGroup | null = null;
  let countyLoading = false;
  let activeClubs = $derived(clubs.filter((club) => club.my_membership_status === 'active'));
  let activeEvents = $derived(events.filter((event) => activeClubs.some((club) => club.id === event.club_id)));

  function scopeParts() { const [kind, id] = scope.split(':', 2); return { kind, id }; }

  function maidenheadCode(latitude: number, longitude: number, precision: 4 | 6) {
    if (latitude < -90 || latitude >= 90 || longitude < -180 || longitude >= 180) return null;
    const normalizedLongitude = longitude + 180;
    const normalizedLatitude = latitude + 90;
    const fieldLongitude = Math.floor(normalizedLongitude / 20);
    const fieldLatitude = Math.floor(normalizedLatitude / 10);
    const squareLongitude = Math.floor((normalizedLongitude % 20) / 2);
    const squareLatitude = Math.floor(normalizedLatitude % 10);
    const code = `${String.fromCharCode(65 + fieldLongitude)}${String.fromCharCode(65 + fieldLatitude)}${squareLongitude}${squareLatitude}`;
    if (precision === 4) return code;
    const subsquareLongitude = Math.floor((normalizedLongitude % 2) / (5 / 60));
    const subsquareLatitude = Math.floor((normalizedLatitude % 1) / (2.5 / 60));
    return `${code}${String.fromCharCode(65 + subsquareLongitude)}${String.fromCharCode(65 + subsquareLatitude)}`;
  }

  function gridCellBounds(grid: string): [[number, number], [number, number]] | null {
    const normalized = grid.trim().toUpperCase();
    if (![4, 6].includes(normalized.length)) return null;
    const bytes = normalized.split('').map((character) => character.charCodeAt(0));
    if (bytes[0] < 65 || bytes[0] > 82 || bytes[1] < 65 || bytes[1] > 82 || bytes[2] < 48 || bytes[2] > 57 || bytes[3] < 48 || bytes[3] > 57) return null;
    let west = -180 + (bytes[0] - 65) * 20 + (bytes[2] - 48) * 2;
    let south = -90 + (bytes[1] - 65) * 10 + (bytes[3] - 48);
    let width = 2;
    let height = 1;
    if (normalized.length === 6) {
      if (bytes[4] < 65 || bytes[4] > 88 || bytes[5] < 65 || bytes[5] > 88) return null;
      west += (bytes[4] - 65) * (5 / 60);
      south += (bytes[5] - 65) * (2.5 / 60);
      width = 5 / 60;
      height = 2.5 / 60;
    }
    return [[south, west], [south + height, west + width]];
  }

  function renderPoints() {
    if (!leaflet || !map || !markerLayer) return;
    markerLayer.clearLayers();
    exactGridLayer?.clearLayers();
    for (const point of points) {
      const bounds = gridCellBounds(point.grid);
      if (bounds && exactGridLayer) {
        exactGridLayer.addLayer(leaflet.rectangle(bounds, {
          color: '#d6feff', weight: 1.4, opacity: 0.95, fill: false, interactive: false,
        }));
      }
      leaflet.circleMarker([point.latitude, point.longitude], {
        radius: Math.min(18, 7 + point.qso_count * 2),
        color: '#07161c',
        weight: 2,
        fillColor: '#64e6f2',
        fillOpacity: 0.9,
      })
        .bindTooltip(`${point.grid} · ${point.qso_count} QSO${point.qso_count === 1 ? '' : 's'}`, { permanent: true, direction: 'top', className: 'map-label grid-label', opacity: 0.95 })
        .on('click', () => { selected = point; if (point.grid.length === 6) map?.setView([point.latitude, point.longitude], 10); })
        .addTo(markerLayer);
    }
    updateFineGrid();
    fitToActivity();
  }

  function fitToActivity() {
    if (!leaflet || !map) return;
    if (points.length === 1) {
      map.setView([points[0].latitude, points[0].longitude], points[0].grid.length === 6 ? 10 : 4);
    } else if (points.length > 1) {
      map.fitBounds(leaflet.latLngBounds(points.map((point) => [point.latitude, point.longitude])), { padding: [28, 28], maxZoom: points.every((point) => point.grid.length === 6) ? 10 : 4 });
    } else {
      map.setView([25, 0], 2);
    }
  }

  function updateLabelVisibility() {
    const zoom = map?.getZoom() ?? 2;
    for (const [layer, minimumZoom] of [[countryLayer, 2], [stateLayer, 3], [countyLayer, 6], [markerLayer, 2]] as const) {
      layer?.eachLayer((item) => {
        const element = item.getTooltip()?.getElement();
        element?.classList.toggle('map-label-hidden', !showLabels || zoom < minimumZoom);
      });
    }
  }

  function applyMapControls() {
    if (!map) return;
    if (showCounties && !countyLayer) void loadCountyLayer();
    for (const [layer, visible] of [
      [countryLayer, showCountries], [stateLayer, showStates],
      [countyLayer, showCounties], [gridFieldsLayer, showGridFields],
      [gridSquaresLayer, showGridSquares], [exactGridLayer, showExactGrids],
      [fineGridLayer, showFineGrids],
    ] as const) {
      if (!layer) continue;
      if (visible && !map.hasLayer(layer)) map.addLayer(layer);
      if (!visible && map.hasLayer(layer)) map.removeLayer(layer);
    }
    updateLabelVisibility();
    updateFineGrid();
  }

  async function loadCountyLayer() {
    if (!leaflet || !map || countyLayer || countyLoading) return;
    countyLoading = true;
    try {
      const countyData = await import('us-atlas/counties-10m.json');
      if (!map || !leaflet) return;
      countyLayer = leaflet.geoJSON(feature(countyData.default as never, 'counties'), {
        style: { color: '#bfd0d2', weight: 0.35, opacity: 0.52, fill: false },
        onEachFeature: (county, layer) => {
          const name = county.properties?.name;
          if (name) layer.bindTooltip(name, { permanent: true, direction: 'center', className: 'map-label county-label', opacity: 0.72 });
        },
      });
      if (showCounties) map.addLayer(countyLayer);
      updateLabelVisibility();
    } catch (cause) {
      notice = `County layer unavailable: ${(cause as Error).message}`;
    } finally {
      countyLoading = false;
    }
  }

  function createGridLayer(longitudeStep: number, latitudeStep: number, color: string, weight: number, opacity: number) {
    if (!leaflet) return null;
    const layer = leaflet.layerGroup();
    for (let longitude = -180; longitude <= 180; longitude += longitudeStep) {
      layer.addLayer(leaflet.polyline([[-85.05, longitude], [85.05, longitude]], { color, weight, opacity, interactive: false }));
    }
    for (let latitude = -80; latitude <= 80; latitude += latitudeStep) {
      layer.addLayer(leaflet.polyline([[latitude, -180], [latitude, 180]], { color, weight, opacity, interactive: false }));
    }
    return layer;
  }

  function updateFineGrid() {
    if (!leaflet || !map || !fineGridLayer) return;
    fineGridLayer.clearLayers();
    if (!showFineGrids || map.getZoom() < 7) return;
    const bounds = map.getBounds();
    const stepLongitude = 5 / 60;
    const stepLatitude = 2.5 / 60;
    const west = Math.max(-180, bounds.getWest());
    const east = Math.min(180, bounds.getEast());
    const south = Math.max(-85, bounds.getSouth());
    const north = Math.min(85, bounds.getNorth());
    const firstLongitude = Math.floor((west + 180) / stepLongitude) * stepLongitude - 180;
    const firstLatitude = Math.floor((south + 90) / stepLatitude) * stepLatitude - 90;
    const longitudeLines = Math.min(360, Math.ceil((east - west) / stepLongitude) + 2);
    const latitudeLines = Math.min(360, Math.ceil((north - south) / stepLatitude) + 2);
    for (let index = 0; index < longitudeLines; index += 1) {
      const longitude = firstLongitude + index * stepLongitude;
      fineGridLayer.addLayer(leaflet.polyline([[south, longitude], [north, longitude]], { color: '#d6feff', weight: 0.35, opacity: 0.48, interactive: false }));
    }
    for (let index = 0; index < latitudeLines; index += 1) {
      const latitude = firstLatitude + index * stepLatitude;
      fineGridLayer.addLayer(leaflet.polyline([[latitude, west], [latitude, east]], { color: '#d6feff', weight: 0.35, opacity: 0.48, interactive: false }));
    }
    if (map.getZoom() < 9 || !showLabels) return;
    const labelStepLongitude = stepLongitude * (map.getZoom() >= 10 ? 4 : 2);
    const labelStepLatitude = stepLatitude * (map.getZoom() >= 10 ? 4 : 2);
    const firstLabelLongitude = Math.floor((west + 180) / labelStepLongitude) * labelStepLongitude - 180;
    const firstLabelLatitude = Math.floor((south + 90) / labelStepLatitude) * labelStepLatitude - 90;
    for (let longitude = firstLabelLongitude; longitude < east; longitude += labelStepLongitude) {
      for (let latitude = firstLabelLatitude; latitude < north; latitude += labelStepLatitude) {
        const code = maidenheadCode(latitude + labelStepLatitude / 2, longitude + labelStepLongitude / 2, 6);
        if (!code) continue;
        fineGridLayer.addLayer(leaflet.marker([latitude + labelStepLatitude / 2, longitude + labelStepLongitude / 2], {
          icon: leaflet.divIcon({ className: 'fine-grid-label', html: code }),
          interactive: false,
        }));
      }
    }
  }

  async function load() {
    loading = true; notice = '';
    try {
      const selectedScope = scopeParts();
      const query = selectedScope.kind === 'overall'
        ? 'scope=overall'
        : `scope=${selectedScope.kind}&scope_id=${encodeURIComponent(selectedScope.id || '')}`;
      points = await api<ActivityMapPoint[]>(`/api/v1/activity/map?${query}`);
      selected = points[0] || null;
      renderPoints();
    } catch (cause) { notice = (cause as Error).message; } finally { loading = false; }
  }

  onMount(() => {
    let disposed = false;
    void import('leaflet').then(async (module) => {
      if (disposed || !mapElement) return;
      leaflet = module;
      let hostedMap: HostedMapConfig | null = null;
      try { hostedMap = await api<HostedMapConfig>('/api/v1/hosted/map-config'); } catch { /* public/community server */ }
      if (disposed || !mapElement) return;
      map = module.map(mapElement, { minZoom: 2, maxZoom: hostedMap?.max_zoom || 12, worldCopyJump: true }).setView([25, 0], 2);
      if (hostedMap?.online_tiles && hostedMap.tile_url && hostedMap.attribution) {
        tileLayer = module.tileLayer(hostedMap.tile_url, {
          attribution: hostedMap.attribution,
          maxZoom: hostedMap.max_zoom,
        }).addTo(map);
      }
      landLayer = module.geoJSON(feature(world as never, 'land'), {
        style: { color: '#59747a', weight: 0.8, fillColor: '#17333a', fillOpacity: hostedMap?.online_tiles ? 0.35 : 0.95 },
      }).addTo(map);
      countryLayer = module.geoJSON(feature(world as never, 'countries'), {
        style: { color: '#76939a', weight: 0.65, fill: false },
        onEachFeature: (country, layer) => {
          const name = country.properties?.name;
          if (name) layer.bindTooltip(name, { permanent: true, direction: 'center', className: 'map-label country-label', opacity: 0.85 });
        },
      }).addTo(map);
      stateLayer = module.geoJSON(feature(usStates as never, 'states'), {
        style: { color: '#9bb5b8', weight: 0.5, opacity: 0.75, fill: false },
        onEachFeature: (state, layer) => {
          const name = state.properties?.name;
          if (name) layer.bindTooltip(name, { permanent: true, direction: 'center', className: 'map-label state-label', opacity: 0.8 });
        },
      }).addTo(map);
      gridFieldsLayer = createGridLayer(20, 10, '#5c929b', 0.7, 0.42);
      gridSquaresLayer = createGridLayer(2, 1, '#75b4bc', 0.45, 0.28);
      exactGridLayer = module.layerGroup().addTo(map);
      fineGridLayer = module.layerGroup().addTo(map);
      markerLayer = module.layerGroup().addTo(map);
      map.on('zoomend', updateLabelVisibility);
      map.on('zoomend moveend', updateFineGrid);
      applyMapControls();
      updateLabelVisibility();
      renderPoints();
    });
    return () => { disposed = true; map?.off('zoomend', updateLabelVisibility); map?.off('zoomend moveend', updateFineGrid); map?.remove(); map = null; tileLayer = null; landLayer = null; countryLayer = null; stateLayer = null; countyLayer = null; gridFieldsLayer = null; gridSquaresLayer = null; exactGridLayer = null; fineGridLayer = null; markerLayer = null; };
  });

  $effect(() => { scope; void load(); });
  $effect(() => { showCountries; showStates; showCounties; showLabels; showGridFields; showGridSquares; showExactGrids; showFineGrids; applyMapControls(); });
</script>

<section class="activity-map">
  <div class="section-head"><div><p class="eyebrow amber">COMMUNITY / QSO MAP</p><h2>Worked grid squares</h2><p class="section-intro">A geographic view of valid Maidenhead grids in QSO exchanges you are allowed to see. Markers are grid-square centres, never precise operator locations.</p></div><b>{points.length} grids</b></div>
  <div class="map-tools"><label>Activity scope<select bind:value={scope}><option value="overall">My owned callsigns</option>{#each identities as identity}<option value={`identity:${identity.id}`}>{identity.callsign} · {identity.identity_type}</option>{/each}{#each activeClubs as club}<option value={`club:${club.id}`}>{club.name} · club activity</option>{/each}{#each activeEvents as event}<option value={`event:${event.id}`}>{event.name} · event activity</option>{/each}</select></label><button class="secondary" onclick={() => void load()} disabled={loading}>{loading ? 'LOADING…' : 'REFRESH MAP'}</button><button class="secondary" onclick={fitToActivity}>FIT TO ACTIVITY</button></div>
  <fieldset class="map-controls"><legend>MAP CONTROLS</legend><label><input type="checkbox" bind:checked={showCountries} /> Country boundaries</label><label><input type="checkbox" bind:checked={showStates} /> U.S. state boundaries</label><label><input type="checkbox" bind:checked={showCounties} /> U.S. county boundaries</label><label><input type="checkbox" bind:checked={showGridFields} /> Maidenhead fields</label><label><input type="checkbox" bind:checked={showGridSquares} /> Maidenhead squares</label><label><input type="checkbox" bind:checked={showFineGrids} /> Six-character grid detail</label><label><input type="checkbox" bind:checked={showExactGrids} /> Contact grid cells</label><label><input type="checkbox" bind:checked={showLabels} /> Labels</label></fieldset>
  <div class="map-frame"><div bind:this={mapElement} class="leaflet-map" aria-label="Geographic QSO activity map"></div>{#if points.length === 0 && !loading}<p class="map-empty">No permitted map-ready exchanges were found. Publish a grid-bearing contact or choose another activity scope.</p>{/if}</div>
  {#if selected}<div class="map-detail"><b>{selected.grid}</b><span>{selected.qso_count} QSO{selected.qso_count === 1 ? '' : 's'} · last {new Date(selected.last_qso_at).toLocaleString()}</span></div>{/if}
  <div class="map-legend"><span><i class="country-line"></i>Country boundaries</span><span><i class="state-line"></i>U.S. state boundaries</span><span><i class="county-line"></i>U.S. county boundaries</span><span><i class="field-line"></i>Maidenhead fields</span><span><i class="square-line"></i>Maidenhead squares</span><span><i class="contact-line"></i>Contact grid cells / six-character detail</span></div>
  <small class="map-attribution">Offline community basemap bundled from Natural Earth 110m data. Labels appear as you zoom in. Grid squares are deliberately approximate.</small>
  {#if notice}<p class="error">{notice}</p>{/if}
</section>

<style>
  .activity-map { display:grid; gap:16px; padding:25px; border:1px solid var(--line); background:var(--panel); margin-bottom:34px; }.section-head { display:flex; justify-content:space-between; gap:16px; align-items:end; }.section-head h2 { margin:4px 0; }.section-head b { color:var(--cyan); font:500 24px ui-monospace; }.map-tools { display:flex; gap:10px; align-items:end; flex-wrap:wrap; }.map-tools label { display:grid; gap:5px; color:var(--muted); font:600 10px ui-monospace; text-transform:uppercase; }.map-controls { display:flex; flex-wrap:wrap; gap:12px 20px; margin:0; padding:10px 12px; border:1px solid var(--line); color:var(--muted); }.map-controls legend { padding:0 6px; color:var(--amber); font:600 10px ui-monospace; letter-spacing:0.08em; }.map-controls label { display:flex; align-items:center; gap:7px; font-size:12px; }.map-controls input { accent-color:var(--cyan); }.map-frame { position:relative; overflow:hidden; border:1px solid var(--line); background:#d8e4e5; }.leaflet-map { height:clamp(340px,55vh,620px); width:100%; }.map-empty { position:absolute; inset:0; z-index:500; display:grid; place-items:center; margin:0; padding:28px; background:#07161ccc; color:var(--muted); text-align:center; pointer-events:none; }.map-detail { display:flex; gap:10px; align-items:baseline; padding:12px; background:var(--chip); }.map-detail b { color:var(--cyan); font:600 18px ui-monospace; }.map-detail span,.map-attribution,.map-legend { color:var(--muted); }.map-legend { display:flex; flex-wrap:wrap; gap:16px; font:600 10px ui-monospace; text-transform:uppercase; }.map-legend span { display:flex; align-items:center; gap:6px; }.map-legend i { display:inline-block; width:18px; border-top:2px solid #76939a; }.map-legend .state-line { border-top-width:1px; border-color:#9bb5b8; }.map-legend .county-line { border-top-width:1px; border-color:#bfd0d2; }.map-legend .field-line { border-top-width:2px; border-color:#5c929b; }.map-legend .square-line { border-top-width:1px; border-color:#75b4bc; }.map-legend .contact-line { border-top-width:2px; border-color:#d6feff; }.map-attribution { display:block; }.error { color:var(--red); margin:0; } :global(.leaflet-container) { font:inherit; } :global(.leaflet-tooltip) { color:#07161c; font-weight:700; } :global(.map-label) { border:1px solid #45636a; border-radius:3px; box-shadow:none; padding:2px 4px; background:#07161ccc; color:#d8edf0; font:600 10px ui-monospace; white-space:nowrap; pointer-events:none; } :global(.state-label) { border-color:#6c898d; color:#eef9fa; font-size:9px; opacity:0.8; } :global(.county-label) { border-color:#8ba5a8; color:#d9e7e9; font-size:8px; opacity:0.7; } :global(.grid-label) { border-color:#d6feff; color:#07161c; background:#d6feffdd; font-size:9px; } :global(.fine-grid-label) { border:1px solid #d6feff; border-radius:2px; box-shadow:none; padding:1px 2px; background:#07161ccc; color:#d6feff; font:600 8px ui-monospace; white-space:nowrap; pointer-events:none; transform:translate(-50%,-50%); } :global(.map-label-hidden) { display:none; } @media (max-width:700px) { .section-head { align-items:start; flex-direction:column; }.map-tools { align-items:stretch; flex-direction:column; }.leaflet-map { height:340px; } }
</style>
