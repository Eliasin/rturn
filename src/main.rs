#![feature(option_result_contains)]

use bevy::{
    input::{mouse::MouseWheel, system::exit_on_esc_system},
    prelude::*,
};

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash, Debug)]
struct GridPosition {
    x: u32,
    y: u32,
}

impl GridPosition {
    fn dist(&self, p: &GridPosition) -> u32 {
        (i32::abs(self.x as i32 - p.x as i32) + i32::abs(self.y as i32 - p.y as i32)) as u32
    }
}

#[derive(PartialEq)]
enum GridHighlightType {
    PlayerUnitMovement,
    PlayerHover,
    PlayerUnitSelected,
}

enum GridAnchorType {
    Center,
    Top,
    Left,
    Right,
    Bottom,
}

#[derive(Bundle)]
struct GridUI {
    grid_position: GridPosition,
    anchor_type: GridAnchorType,
    sprite_size: SpriteSize,
    transform: Transform,
    mouse_interactible: MouseInteractible,
}

struct SelectedUnit;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
enum Turn {
    Player,
    Enemy,
    Neutral,
}

impl Default for Turn {
    fn default() -> Self {
        Turn::Player
    }
}

struct TurnState {
    turn: Turn,
}

#[derive(Default)]
struct LastClick {
    was_handled: bool,
}

struct GridHighlight {
    pos: GridPosition,
    highlight_type: GridHighlightType,
}

struct GameGrid {
    width: usize,
    height: usize,
}

#[derive(Default)]
struct SpriteSize {
    x: f32,
    y: f32,
    render_scale: f32,
}

impl SpriteSize {
    pub fn new(x: f32, y: f32) -> Self {
        SpriteSize {
            x,
            y,
            render_scale: 1.,
        }
    }

    pub fn new_with_render_size(x: f32, y: f32, render_scale: f32) -> Self {
        SpriteSize { x, y, render_scale }
    }
}

#[derive(Default, Copy, Clone)]
struct AnimationRange {
    start_index: u32,
    end_index: u32,
    current_index: u32,
}

impl AnimationRange {
    fn from_start_end(start_index: u32, end_index: u32) -> Self {
        AnimationRange {
            start_index,
            end_index,
            current_index: start_index,
        }
    }

    fn reset(&mut self) {
        self.current_index = self.start_index;
    }

    fn advance(&mut self, should_loop: bool) {
        if self.current_index == self.end_index {
            if should_loop {
                self.reset();
            }
        } else {
            self.current_index += 1;
        }
    }
}

#[derive(Default)]
struct IdleAnimation {
    animation: Option<AnimationRange>,
    should_loop: bool,
    timer: Timer,
}

#[derive(Default)]
struct SelectedAnimation {
    animation: Option<AnimationRange>,
    should_loop: bool,
    timer: Timer,
}

#[derive(Default)]
struct MouseInteractible {
    bounding_box: Rect<f32>,
    z: u32,
}

impl MouseInteractible {
    fn from_z(z: u32) -> Self {
        MouseInteractible {
            z,
            ..Default::default()
        }
    }
}

#[derive(Default)]
struct Clickable {
    clicked: bool,
}

#[derive(Default)]
struct Hoverable {
    hovered: bool,
}

#[derive(Bundle, Debug)]
struct GridEntity {
    grid_pos: GridPosition,
}

struct ChangeSpriteIndexOnHover {
    default_index: u32,
    hover_index: u32,
}

#[derive(Default)]
struct GridTileTag;

#[derive(Bundle, Default)]
struct GridTile {
    grid_pos: GridPosition,
    #[bundle]
    sprite: SpriteSheetBundle,
    sprite_size: SpriteSize,
    grid_tile_tag: GridTileTag,
    mouse_interactible: MouseInteractible,
    clickable: Clickable,
    hoverable: Hoverable,
}

struct MovementRange {
    range: u32,
    flying: bool,
}

struct Selectable;

#[derive(Bundle)]
struct PlayerUnit {
    #[bundle]
    grid_entity: GridEntity,
    #[bundle]
    sprite: SpriteSheetBundle,
    sprite_size: SpriteSize,
    mouse_interactible: MouseInteractible,
    hoverable: Hoverable,
    clickable: Clickable,
    selectable: Selectable,
}

struct SpriteSheets {
    grid: Handle<TextureAtlas>,
    myrrh: Handle<TextureAtlas>,
}

struct RenderSettings {
    tile_size: f32,
    tile_scale: f32,
    camera_offset: Vec2,
}

fn main() {
    App::build()
        .add_startup_system(setup.system())
        .insert_resource(WindowDescriptor {
            title: "Rturn".to_string(),
            width: 1200.,
            height: 800.,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_stage(
            "texture_setup",
            SystemStage::single(setup_textures.system()),
        )
        .add_startup_stage(
            "world_setup",
            SystemStage::parallel()
                .with_system(setup_grid_tiles.system())
                .with_system(spawn_units.system()),
        )
        .add_system(move_camera.system())
        .add_system(handle_mouse_interactions.system().label("mouse_input"))
        .add_system(handle_hover_sprite_change.system().after("mouse_input"))
        .add_system(
            handle_player_unit_selection_grid_highlights
                .system()
                .label("unit_selection_grid_highlights")
                .after("unit_selection"),
        )
        .add_system(
            handle_player_unit_selection_movement_highlights
                .system()
                .label("unit_selection_movment_highlights")
                .after("unit_selection"),
        )
        .add_system(
            handle_unit_selection
                .system()
                .label("unit_selection")
                .after("mouse_input")
                .after("handle_grid_clicks"),
        )
        .add_system(
            handle_hover_grid_highlights
                .system()
                .label("grid_hover_highlight")
                .after("mouse_input"),
        )
        .add_system(
            render_grid_tiles
                .system()
                .after("unit_selection_grid_highlights")
                .after("unit_selection_movment_highlights"),
        )
        .add_system(handle_grid_clicks.system().label("handle_grid_clicks"))
        .add_system(exit_on_esc_system.system())
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(render_grid_objects.system().label("render_grid_objects"))
                .with_system(animate_idle.system().after("render_grid_objects"))
                .with_system(animate_selected.system().after("render_grid_objects")),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.insert_resource(GameGrid {
        width: 16,
        height: 16,
    });
    commands.insert_resource(RenderSettings {
        tile_size: 64.,
        tile_scale: 2.,
        camera_offset: Vec2::new(0., 0.),
    });
    commands.insert_resource(LastClick::default());
    commands.insert_resource(TurnState { turn: Turn::Player });
}

fn setup_textures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let grid_texture_handle = asset_server.load("textures/grid.png");
    let grid_texture_atlas =
        TextureAtlas::from_grid(grid_texture_handle, Vec2::new(32.0, 32.0), 4, 2);
    let grid_texture_atlas_handle = texture_atlases.add(grid_texture_atlas);

    let myrrh_texture_handle = asset_server.load("textures/myrrh.png");
    let myrrh_texture_atlas =
        TextureAtlas::from_grid(myrrh_texture_handle, Vec2::new(128.0, 128.0), 3, 3);
    let myrrh_texture_atlas_handle = texture_atlases.add(myrrh_texture_atlas);

    commands.insert_resource(SpriteSheets {
        grid: grid_texture_atlas_handle,
        myrrh: myrrh_texture_atlas_handle,
    });
}

fn setup_grid_tiles(
    mut commands: Commands,
    sprite_sheets: Res<SpriteSheets>,
    game_grid: Res<GameGrid>,
) {
    let sprite = SpriteSheetBundle {
        texture_atlas: sprite_sheets.grid.clone(),
        sprite: TextureAtlasSprite::new(2),
        ..Default::default()
    };

    for x in 0..game_grid.width {
        for y in 0..game_grid.height {
            let grid_pos = GridPosition {
                x: x as u32,
                y: y as u32,
            };

            let sprite = sprite.clone();

            commands.spawn_bundle(GridTile {
                grid_pos,
                sprite,
                sprite_size: SpriteSize::new(32., 32.),
                grid_tile_tag: GridTileTag {},
                ..Default::default()
            });
        }
    }
}

fn spawn_units(mut commands: Commands, sprite_sheets: Res<SpriteSheets>) {
    commands
        .spawn_bundle(PlayerUnit {
            grid_entity: GridEntity {
                grid_pos: GridPosition { x: 4, y: 4 },
            },
            sprite: SpriteSheetBundle {
                texture_atlas: sprite_sheets.myrrh.clone(),
                sprite: TextureAtlasSprite::new(0),
                ..Default::default()
            },
            sprite_size: SpriteSize::new_with_render_size(128., 128., 1.5),
            mouse_interactible: MouseInteractible::from_z(10),
            clickable: Clickable::default(),
            hoverable: Hoverable::default(),
            selectable: Selectable {},
        })
        .insert(MovementRange {
            range: 3,
            flying: false,
        })
        .insert(IdleAnimation {
            animation: Some(AnimationRange::from_start_end(0, 1)),
            should_loop: true,
            timer: Timer::from_seconds(0.2, true),
        })
        .insert(SelectedAnimation {
            animation: Some(AnimationRange::from_start_end(0, 7)),
            should_loop: false,
            timer: Timer::from_seconds(0.1, true),
        });
}

fn render_grid_objects(
    render_settings: Res<RenderSettings>,
    mut q: Query<(
        &GridPosition,
        &SpriteSize,
        &mut Transform,
        Option<&GridEntity>,
        Option<&mut MouseInteractible>,
    )>,
    grid_highlight_query: Query<&GridHighlight>,
) {
    let RenderSettings {
        tile_size,
        tile_scale,
        camera_offset,
    } = *render_settings;

    let mut need_movement_z_level = vec![];
    let mut need_selected_z_level = vec![];

    for grid_highlight in grid_highlight_query.iter() {
        use GridHighlightType::*;
        match grid_highlight.highlight_type {
            PlayerUnitSelected => {
                need_selected_z_level.push(grid_highlight.pos);
            }
            _ => {
                need_movement_z_level.push(grid_highlight.pos);
            }
        }
    }

    for (pos, sprite_size, mut transform, grid_entity, mouse_interactible) in q.iter_mut() {
        let z = if grid_entity.is_some() {
            10.
        } else if need_selected_z_level.contains(pos) {
            9.
        } else if need_movement_z_level.contains(pos) {
            5.
        } else {
            1.
        };

        let x_scale = tile_size / sprite_size.x * tile_scale;
        let y_scale = tile_size / sprite_size.y * tile_scale;

        let x_adjustment = pos.x as f32 * tile_size * tile_scale / 16.;
        let y_adjustment = pos.y as f32 * tile_size * tile_scale / 16.;

        let center_x = camera_offset.x + tile_size * tile_scale * pos.x as f32 - x_adjustment;
        let center_y = camera_offset.y + tile_size * tile_scale * pos.y as f32 - y_adjustment;

        transform.translation = Vec3::new(center_x, center_y, z);

        transform.scale = Vec3::new(
            x_scale * sprite_size.render_scale,
            y_scale * sprite_size.render_scale,
            1.,
        );

        if let Some(mut mouse_interactible) = mouse_interactible {
            mouse_interactible.bounding_box = Rect::<f32> {
                top: center_y + (tile_size / 4.) * y_scale - 1.,
                bottom: center_y - (tile_size / 4.) * y_scale - 1.,
                right: center_x + (tile_size / 4.) * x_scale - 1.,
                left: center_x - (tile_size / 4.) * x_scale - 1.,
            };
        }
    }
}

fn move_camera(
    keyboard_input: Res<Input<KeyCode>>,
    mut ev_scroll: EventReader<MouseWheel>,
    mut render_settings: ResMut<RenderSettings>,
) {
    if keyboard_input.pressed(KeyCode::Left) {
        render_settings.camera_offset.x += 16.;
    }
    if keyboard_input.pressed(KeyCode::Right) {
        render_settings.camera_offset.x -= 16.;
    }
    if keyboard_input.pressed(KeyCode::Up) {
        render_settings.camera_offset.y -= 16.;
    }
    if keyboard_input.pressed(KeyCode::Down) {
        render_settings.camera_offset.y += 16.;
    }

    const MOUSE_SCROLL_SENSITIVITY: f32 = 0.2;
    for ev in ev_scroll.iter() {
        render_settings.tile_scale += ev.y * MOUSE_SCROLL_SENSITIVITY;

        render_settings.tile_scale = render_settings.tile_scale.max(1.);
        render_settings.tile_scale = render_settings.tile_scale.min(10.);
    }
}

trait ContainsPoint {
    fn contains_point(&self, p: Vec2) -> bool;
}

impl ContainsPoint for Rect<f32> {
    fn contains_point(&self, p: Vec2) -> bool {
        p.x < self.right && p.x > self.left && p.y > self.bottom && p.y < self.top
    }
}

fn handle_mouse_interactions(
    mouse_input: Res<Input<MouseButton>>,
    mut q: Query<(
        Entity,
        &MouseInteractible,
        Option<&mut Hoverable>,
        Option<&mut Clickable>,
    )>,
    windows: Res<Windows>,
    mut last_click: ResMut<LastClick>,
) {
    let window = windows.get_primary().unwrap();

    if let Some(mut position) = window.cursor_position() {
        let clicked = mouse_input.just_pressed(MouseButton::Left);

        position.x -= window.width() / 2.;
        position.y -= window.height() / 2.;

        let mut click_handled = false;

        let mut highest_z_clicked: Option<(u32, Entity)> = None;
        for (entity, mouse_interactible, hoverable, clickable) in q.iter_mut() {
            if mouse_interactible.bounding_box.contains_point(position) {
                if clicked {
                    match highest_z_clicked {
                        Some((z, _)) => {
                            if mouse_interactible.z > z {
                                highest_z_clicked = Some((mouse_interactible.z, entity));
                            }
                        }
                        None => {
                            highest_z_clicked = Some((mouse_interactible.z, entity));
                        }
                    }
                } else {
                    if let Some(mut hoverable) = hoverable {
                        hoverable.hovered = true;
                    }
                    if let Some(mut clickable) = clickable {
                        clickable.clicked = false;
                    }
                }
            } else {
                if let Some(mut hoverable) = hoverable {
                    hoverable.hovered = false;
                }
                if let Some(mut clickable) = clickable {
                    clickable.clicked = false;
                }
            }
        }

        if let Some((_, entity)) = highest_z_clicked {
            let (_, _, hoverable, clickable) = q.get_mut(entity).unwrap();
            if let Some(mut hoverable) = hoverable {
                hoverable.hovered = false;
            }
            if let Some(mut clickable) = clickable {
                clickable.clicked = true;
            }
            click_handled = true;
        }

        if clicked {
            last_click.was_handled = click_handled;
        }
    }
}

fn handle_hover_sprite_change(
    mut q: Query<(
        &ChangeSpriteIndexOnHover,
        &Hoverable,
        &mut TextureAtlasSprite,
    )>,
) {
    for (change_sprite_on_hover, hoverable, mut texture_atlas_sprite) in q.iter_mut() {
        if hoverable.hovered {
            *texture_atlas_sprite = TextureAtlasSprite::new(change_sprite_on_hover.hover_index);
        } else {
            *texture_atlas_sprite = TextureAtlasSprite::new(change_sprite_on_hover.default_index);
        }
    }
}

fn handle_player_unit_selection_grid_highlights(
    mut commands: Commands,
    grid_tile_query: Query<&GridPosition, With<GridTileTag>>,
    grid_highlight_query: Query<(Entity, &GridHighlight)>,
    selected_unit_query: Query<&GridPosition, With<SelectedUnit>>,
) {
    let mut selected_player_unit_highlights = vec![];
    for (entity, grid_highlight) in grid_highlight_query.iter() {
        match grid_highlight.highlight_type {
            GridHighlightType::PlayerUnitSelected => {
                selected_player_unit_highlights.push((entity, grid_highlight.pos));
            }
            _ => {}
        }
    }

    if let Ok(selected_position) = selected_unit_query.single() {
        let mut new_selected_tile = None;
        for grid_position in grid_tile_query.iter() {
            if *selected_position == *grid_position {
                new_selected_tile = Some(*grid_position);
            }
        }

        let mut need_spawn_new_highlight = true;
        if let Some(new_selected_tile) = new_selected_tile {
            for (entity, grid_pos) in selected_player_unit_highlights.into_iter() {
                if grid_pos != new_selected_tile {
                    commands.entity(entity).despawn();
                } else {
                    need_spawn_new_highlight = false;
                }
            }

            if need_spawn_new_highlight {
                commands.spawn().insert(GridHighlight {
                    pos: new_selected_tile,
                    highlight_type: GridHighlightType::PlayerUnitSelected,
                });
            }
        }
    } else {
        for (entity, _pos) in selected_player_unit_highlights {
            commands.entity(entity).despawn();
        }
    }
}

fn render_grid_tiles(
    grid_highlight_query: Query<&GridHighlight>,
    mut grid_tile_query: Query<(&mut TextureAtlasSprite, &GridPosition), With<GridTileTag>>,
) {
    let mut player_unit_selected = vec![];
    let mut player_unit_movement = vec![];
    let mut player_hover = vec![];

    for grid_highlight in grid_highlight_query.iter() {
        use GridHighlightType::*;
        match grid_highlight.highlight_type {
            PlayerUnitSelected => player_unit_selected.push(grid_highlight.pos),
            PlayerUnitMovement => player_unit_movement.push(grid_highlight.pos),
            PlayerHover => player_hover.push(grid_highlight.pos),
        };
    }

    for (mut texture_atlas_sprite, grid_position) in grid_tile_query.iter_mut() {
        if player_unit_selected.contains(&grid_position) {
            *texture_atlas_sprite = TextureAtlasSprite::new(0);
        } else if player_unit_movement.contains(&grid_position) {
            *texture_atlas_sprite = TextureAtlasSprite::new(3);
        } else if player_hover.contains(&grid_position) {
            *texture_atlas_sprite = TextureAtlasSprite::new(1);
        } else {
            *texture_atlas_sprite = TextureAtlasSprite::new(2);
        }
    }
}

fn handle_unit_selection(
    mut commands: Commands,
    mut clickable_player_unit_query: Query<
        (Entity, &Clickable, Option<&mut SelectedAnimation>),
        With<Selectable>,
    >,
    mut selected_unit_query: Query<(Entity, Option<&mut IdleAnimation>), With<SelectedUnit>>,
    last_click: Res<LastClick>,
) {
    let mut remove_all_currently_selected = false;
    for (entity, clickable, mut selected_animation) in clickable_player_unit_query.iter_mut() {
        if clickable.clicked {
            commands.entity(entity).insert(SelectedUnit {});
            remove_all_currently_selected = true;

            if let Some(mut selected_animation) =
                selected_animation.as_mut().map(|s| s.animation).flatten()
            {
                selected_animation.current_index = selected_animation.start_index;
            }
            break;
        }
    }

    if !last_click.was_handled {
        remove_all_currently_selected = true;
    }

    if remove_all_currently_selected {
        for (entity, idle_animation) in selected_unit_query.iter_mut() {
            commands.entity(entity).remove::<SelectedUnit>();
        }
    }
}

fn handle_player_unit_selection_movement_highlights(
    mut commands: Commands,
    selected_unit_query: Query<&GridPosition, With<SelectedUnit>>,
    grid_tile_query: Query<&GridPosition, With<GridTileTag>>,
    grid_highlight_query: Query<(Entity, &GridHighlight)>,
    player_unit_query: Query<(&GridPosition, &MovementRange)>,
) {
    let mut selected_unit_movement_highlights = vec![];
    for (entity, grid_highlight) in grid_highlight_query.iter() {
        match grid_highlight.highlight_type {
            GridHighlightType::PlayerUnitMovement => {
                selected_unit_movement_highlights.push((entity, grid_highlight.pos));
            }
            _ => {}
        }
    }

    if let Ok(selected_player_unit_pos) = selected_unit_query.single() {
        let mut selected_unit_movement = None;
        for (pos, movement_range) in player_unit_query.iter() {
            if *pos == *selected_player_unit_pos {
                selected_unit_movement = Some(movement_range);
                break;
            }
        }

        if let Some(selected_unit_movement) = selected_unit_movement {
            let mut tiles_need_highlight = vec![];
            for grid_position in grid_tile_query.iter() {
                if grid_position.dist(&selected_player_unit_pos) <= selected_unit_movement.range
                    && grid_position.dist(&selected_player_unit_pos) > 0
                {
                    tiles_need_highlight.push(*grid_position);
                }
            }

            for (entity, pos) in selected_unit_movement_highlights.iter() {
                if !tiles_need_highlight.contains(pos) {
                    commands.entity(*entity).despawn();
                }
            }

            for pos in tiles_need_highlight {
                if !selected_unit_movement_highlights
                    .iter()
                    .map(|(_, p)| *p)
                    .collect::<Vec<GridPosition>>()
                    .contains(&pos)
                {
                    commands.spawn().insert(GridHighlight {
                        pos,
                        highlight_type: GridHighlightType::PlayerUnitMovement,
                    });
                }
            }
        }
    } else {
        for (entity, _) in selected_unit_movement_highlights {
            commands.entity(entity).despawn();
        }
    }
}

fn handle_hover_grid_highlights(
    mut commands: Commands,
    grid_tile_query: Query<(&GridPosition, &Hoverable), With<GridTileTag>>,
    grid_highlight_query: Query<(Entity, &GridHighlight)>,
) {
    let mut hover_highlights = vec![];
    for (entity, grid_highlight) in grid_highlight_query.iter() {
        match grid_highlight.highlight_type {
            GridHighlightType::PlayerHover => {
                hover_highlights.push((entity, grid_highlight.pos));
            }
            _ => {}
        }
    }

    let mut hovered_tiles = vec![];
    for (pos, hoverable) in grid_tile_query.iter() {
        if hoverable.hovered {
            hovered_tiles.push(*pos);
        }
    }

    for (entity, pos) in hover_highlights.iter() {
        if !hovered_tiles.contains(pos) {
            commands.entity(*entity).despawn();
        }
    }

    for pos in hovered_tiles {
        if !hover_highlights
            .iter()
            .map(|(_, p)| *p)
            .collect::<Vec<GridPosition>>()
            .contains(&pos)
        {
            commands.spawn().insert(GridHighlight {
                pos,
                highlight_type: GridHighlightType::PlayerHover,
            });
        }
    }
}

fn handle_grid_clicks(
    mut commands: Commands,
    grid_highlight_query: Query<&GridHighlight>,
    grid_tile_query: Query<(&Clickable, &GridPosition), With<GridTileTag>>,
    mut selected_unit_query: Query<
        (Entity, &mut GridPosition),
        (With<SelectedUnit>, Without<GridTileTag>),
    >,
) {
    if let Ok((entity, mut selected_player_unit_pos)) = selected_unit_query.single_mut() {
        let movement_highlight_positions = grid_highlight_query
            .iter()
            .filter(|grid_highlight| {
                grid_highlight.highlight_type == GridHighlightType::PlayerUnitMovement
            })
            .map(|grid_highlight| grid_highlight.pos)
            .collect::<Vec<GridPosition>>();

        for (clickable, pos) in grid_tile_query.iter() {
            if clickable.clicked && movement_highlight_positions.contains(pos) {
                *selected_player_unit_pos = *pos;
                commands.entity(entity).remove::<SelectedUnit>();
                break;
            } else if clickable.clicked {
                commands.entity(entity).remove::<SelectedUnit>();
                break;
            }
        }
    }
}

fn animate_idle(
    mut idle_animation_query: Query<
        (&mut TextureAtlasSprite, &mut IdleAnimation),
        Without<SelectedUnit>,
    >,
    time: Res<Time>,
) {
    for (mut texture_atlas_sprite, mut idle_animation) in idle_animation_query.iter_mut() {
        if idle_animation.timer.tick(time.delta()).just_finished() {
            let should_loop = idle_animation.should_loop;
            if let Some(animation) = idle_animation.animation.as_mut() {
                *texture_atlas_sprite = TextureAtlasSprite::new(animation.current_index);
                animation.advance(should_loop);
            }
        }
    }
}

fn animate_selected(
    mut selected_animation_query: Query<
        (&mut TextureAtlasSprite, &mut SelectedAnimation),
        With<SelectedUnit>,
    >,
    time: Res<Time>,
) {
    for (mut texture_atlas_sprite, mut selected_animation) in selected_animation_query.iter_mut() {
        if selected_animation.timer.tick(time.delta()).just_finished() {
            let should_loop = selected_animation.should_loop;
            if let Some(animation) = selected_animation.animation.as_mut() {
                *texture_atlas_sprite = TextureAtlasSprite::new(animation.current_index);
                animation.advance(should_loop);
            }
        }
    }
}
