import Phaser from 'phaser';
import { toCityViewModel } from './toCityViewModel';
import type { CityLocationView, CityMapFeature, CityViewModel, PublicScenario, PublicState, VisualEvent } from './types';

export type CityRendererOptions = {
  parent: HTMLElement;
  scenario: PublicScenario;
  state: PublicState;
  visualEvent?: VisualEvent | null;
  locked?: boolean;
  onAction: (actionId: string) => void;
};

function color(value: string, fallback: number): number {
  const normalized = value.trim().replace(/^#/, '');
  return /^[0-9a-f]{6}$/i.test(normalized) ? Number.parseInt(normalized, 16) : fallback;
}

class CivicCityScene extends Phaser.Scene {
  private readonly view: CityViewModel;
  private readonly onAction: (actionId: string) => void;
  private readonly locked: boolean;
  private readonly nodes = new Map<string, Phaser.GameObjects.Container>();
  private reducedMotion = false;

  constructor(options: CityRendererOptions) {
    super({ key: 'CivicCityScene' });
    this.view = toCityViewModel(options.scenario, options.state, options.visualEvent ?? null);
    this.onAction = options.onAction;
    this.locked = Boolean(options.locked);
  }

  create() {
    this.reducedMotion = globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
    this.drawWorld();
    this.view.locations.forEach((location, index) => this.drawLocation(location, index));
    this.drawPlayer();
    this.playVisualEvent();
  }

  private drawWorld() {
    const primary = color(this.view.palette.primary, 0x172c3f);
    const accent = color(this.view.palette.accent, 0xbb6b22);
    const background = color(this.view.palette.background, 0xf4f1e9);
    this.cameras.main.setBackgroundColor(background);
    const graphics = this.add.graphics();
    graphics.fillStyle(primary, 0.08).fillRoundedRect(24, 24, 952, 552, 28);
    if (this.view.mapPlan) this.drawCityPlan(graphics);
    else this.drawAbstractPlan(graphics, primary);

    graphics.fillStyle(0xffffff, 0.88).fillRoundedRect(30, 28, 940, 55, 14);
    this.view.factors.forEach((factor, index) => {
      const x = 44 + index * 155;
      const range = Math.max(1, factor.max - factor.min);
      const ratio = Math.max(0, Math.min(1, (factor.value - factor.min) / range));
      graphics.fillStyle(primary, 0.12).fillRoundedRect(x, 62, 138, 7, 4);
      graphics.fillStyle(accent, 0.7).fillRoundedRect(x, 62, 138 * ratio, 7, 4);
      this.add.text(x, 40, factor.label, {
        color: this.view.palette.primary,
        fontFamily: 'system-ui, sans-serif',
        fontSize: '12px',
        fontStyle: 'bold',
        fixedWidth: 138,
      });
    });
  }

  private drawAbstractPlan(graphics: Phaser.GameObjects.Graphics, primary: number) {
    graphics.lineStyle(18, 0xffffff, 0.72);
    for (let offset = -120; offset < 1100; offset += 190) {
      graphics.lineBetween(offset, 520, offset + 330, 70);
    }
    graphics.lineStyle(9, primary, 0.13);
    for (let y = 115; y < 560; y += 145) graphics.lineBetween(35, y, 965, y - 45);
    graphics.fillStyle(0x4b9ac1, 0.35).fillRect(-30, 348, 1060, 38);
  }

  private drawFeatureLines(
    graphics: Phaser.GameObjects.Graphics,
    features: CityMapFeature[],
    width: number,
    stroke: number,
    alpha: number,
  ) {
    graphics.lineStyle(width, stroke, alpha);
    for (const feature of features) {
      if (feature.points.length < 2) continue;
      graphics.beginPath();
      graphics.moveTo(feature.points[0][0], feature.points[0][1]);
      for (let index = 1; index < feature.points.length; index += 1) {
        graphics.lineTo(feature.points[index][0], feature.points[index][1]);
      }
      graphics.strokePath();
    }
  }

  private drawCityPlan(graphics: Phaser.GameObjects.Graphics) {
    const plan = this.view.mapPlan;
    if (!plan) return;
    const features = plan.features;
    this.drawFeatureLines(graphics, features.filter((feature) => feature.kind === 'water'), 5, 0x54a8ca, 0.7);
    this.drawFeatureLines(graphics, features.filter((feature) => feature.kind === 'rail'), 2, 0x57636b, 0.32);
    const roads = (roadClass: string) => features.filter(
      (feature) => feature.kind === 'road' && feature.class === roadClass,
    );
    for (const [roadClass, width] of [['secondary', 2], ['primary', 4], ['trunk', 5], ['motorway', 6]] as const) {
      const layer = roads(roadClass);
      this.drawFeatureLines(graphics, layer, width + 3, 0xffffff, 0.86);
      this.drawFeatureLines(graphics, layer, width, roadClass === 'motorway' ? 0xd89b42 : 0xc2aa7d, 0.72);
    }
    const labelPositions: [number, number][] = [];
    const seenNames = new Set<string>();
    for (const feature of features) {
      if (
        labelPositions.length >= 6
        || feature.kind !== 'road'
        || feature.class === 'secondary'
        || !feature.name
        || seenNames.has(feature.name)
      ) continue;
      const point = feature.points[Math.floor(feature.points.length / 2)];
      if (!point || point[1] < 105 || labelPositions.some(([x, y]) => Math.hypot(point[0] - x, point[1] - y) < 135)) {
        continue;
      }
      seenNames.add(feature.name);
      labelPositions.push(point);
      this.add.text(point[0], point[1], feature.name.slice(0, 30), {
        color: '#425058',
        backgroundColor: 'rgba(255,255,255,.72)',
        fontFamily: 'system-ui, sans-serif',
        fontSize: '10px',
        padding: { x: 3, y: 2 },
      }).setOrigin(0.5);
    }
    this.add.text(950, 548, plan.label, {
      color: '#27343b',
      backgroundColor: 'rgba(255,255,255,.82)',
      fontFamily: 'system-ui, sans-serif',
      fontSize: '13px',
      fontStyle: 'bold',
      padding: { x: 9, y: 5 },
    }).setOrigin(1, 1);
  }

  private drawLocation(location: CityLocationView, index: number) {
    const x = 55 + location.x * 8.9;
    const y = 72 + location.y * 4.65;
    const primary = color(this.view.palette.primary, 0x172c3f);
    const accent = color(this.view.palette.accent, 0xbb6b22);
    const container = this.add.container(x, y);
    const building = this.add.graphics();
    const border = location.enabled ? accent : primary;
    building.fillStyle(0xfffdf8, 0.96).lineStyle(location.enabled ? 4 : 2, border, location.actionCount ? 0.95 : 0.45);
    building.fillRoundedRect(-58, -29, 116, 58, 12).strokeRoundedRect(-58, -29, 116, 58, 12);
    building.fillStyle(primary, 0.12 + (index % 3) * 0.04).fillRect(-44, -16, 25, 31).fillRect(-10, -16, 20, 31).fillRect(19, -16, 25, 31);
    const label = this.add.text(0, 38, location.label, {
      align: 'center',
      color: this.view.palette.primary,
      fontFamily: 'system-ui, sans-serif',
      fontSize: '13px',
      fontStyle: 'bold',
      wordWrap: { width: 138 },
    }).setOrigin(0.5, 0);
    container.add([building, label]);
    if (location.enabled && location.actionId && !this.locked) {
      const zone = this.add.zone(0, 0, 126, 70).setInteractive({ useHandCursor: true });
      zone.on('pointerover', () => container.setScale(1.07));
      zone.on('pointerout', () => container.setScale(1));
      zone.on('pointerdown', () => this.onAction(location.actionId as string));
      container.add(zone);
      if (!this.reducedMotion) {
        this.tweens.add({ targets: building, alpha: 0.68, yoyo: true, repeat: -1, duration: 1050 + index * 70 });
      }
    } else if (location.actionCount > 0) {
      container.setAlpha(0.62);
    }
    this.nodes.set(location.id, container);
  }

  private drawPlayer() {
    const target = this.view.playerLocationId ? this.nodes.get(this.view.playerLocationId) : null;
    if (!target) return;
    const marker = this.add.container(target.x - 66, target.y - 42);
    const body = this.add.circle(0, 7, 13, color(this.view.palette.accent, 0xbb6b22), 1).setStrokeStyle(3, 0xffffff, 1);
    const head = this.add.circle(0, -10, 8, 0x7c5034, 1).setStrokeStyle(2, 0xffffff, 1);
    marker.add([body, head]);
    if (!this.reducedMotion) this.tweens.add({ targets: marker, y: marker.y - 5, yoyo: true, repeat: -1, duration: 900 });
  }

  private playVisualEvent() {
    const event = this.view.visualEvent;
    if (!event) return;
    const target = event.focus_location_id ? this.nodes.get(event.focus_location_id) : null;
    if (target && !this.reducedMotion) {
      this.tweens.add({ targets: target, scale: 1.22, yoyo: true, repeat: 2, duration: 240 });
    }
    const effectColor = event.effect === 'danger' || event.effect === 'setback'
      ? 0xb83a34
      : event.effect === 'support' || event.effect === 'success'
        ? 0x34845e
        : color(this.view.palette.accent, 0xbb6b22);
    const flash = this.add.rectangle(500, 300, 1000, 600, effectColor, this.reducedMotion ? 0.08 : 0.18);
    if (!this.reducedMotion) this.tweens.add({ targets: flash, alpha: 0, duration: 650, onComplete: () => flash.destroy() });
    else flash.setAlpha(0.05);
    if (event.animation) {
      const caption = this.add.text(500, 540, event.animation.replaceAll('-', ' '), {
        color: '#ffffff',
        backgroundColor: `#${effectColor.toString(16).padStart(6, '0')}`,
        fontFamily: 'system-ui, sans-serif',
        fontSize: '16px',
        fontStyle: 'bold',
        padding: { x: 13, y: 8 },
      }).setOrigin(0.5);
      if (!this.reducedMotion) this.tweens.add({ targets: caption, alpha: 0, delay: 850, duration: 350 });
    }
  }
}

export class CityRenderer {
  private readonly game: Phaser.Game;

  constructor(options: CityRendererOptions) {
    const scene = new CivicCityScene(options);
    this.game = new Phaser.Game({
      type: Phaser.AUTO,
      parent: options.parent,
      width: 1000,
      height: 600,
      transparent: true,
      backgroundColor: 'rgba(0,0,0,0)',
      scene: [scene],
      render: { antialias: true, pixelArt: false },
      scale: {
        mode: Phaser.Scale.FIT,
        autoCenter: Phaser.Scale.CENTER_BOTH,
        width: 1000,
        height: 600,
      },
      input: { keyboard: false, gamepad: false, touch: true, mouse: true },
      banner: false,
    });
  }

  destroy() {
    this.game.destroy(true);
  }
}
