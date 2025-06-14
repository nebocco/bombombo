use bevy::prelude::*;

use crate::{
    PausableSystems,
    audio::{SEVolume, SoundEffectAssets, sound_effect},
    gameplay::{
        CurrentLevel, FireAnimation, GamePhase, GridCoord, Item, ItemAssets, ItemState,
        init_level::{GridTile, LevelBase},
    },
    screens::Screen,
    theme::{UiAssets, widget},
};

pub(super) fn plugin(app: &mut App) {
    // app.register_type::<Item>();

    // app.register_type::<ItemAssets>();

    app.init_resource::<SelectedItem>()
        .init_resource::<CurrentPlacement>();

    app.add_observer(create_object);
    // .add_observer(try_create_single_fire);

    app.add_systems(
        OnEnter(GamePhase::Edit),
        (
            spawn_item_buttons,
            spawn_controlflow_buttons,
            init_edit_state,
        ),
    )
    .add_systems(OnExit(Screen::Gameplay), reset_current_placement)
    .add_systems(OnEnter(GamePhase::Edit), apply_current_placement)
    .add_systems(
        Update,
        (reset_all_object_placements, run_simulation_with_keyboard)
            .run_if(in_state(GamePhase::Edit))
            .in_set(PausableSystems),
    )
    .add_systems(
        Update,
        highlight_selected_item
            .run_if(in_state(GamePhase::Edit).and(resource_changed::<SelectedItem>)),
    );
}

fn spawn_item_buttons(
    mut commands: Commands,
    item_assets: Res<ItemAssets>,
    ui_assets: Res<UiAssets>,
) {
    commands
        .spawn((
            widget::ui_root("Item Buttons"),
            GlobalZIndex(0),
            LevelBase,
            StateScoped(Screen::Gameplay),
            children![
                widget::item_button(
                    Handle::clone(&item_assets.sprite_sheet),
                    &ui_assets,
                    Handle::clone(&item_assets.texture_atlas_layout),
                    Item::BombSmall,
                    select_item::<0>
                ),
                widget::item_button(
                    Handle::clone(&item_assets.sprite_sheet),
                    &ui_assets,
                    Handle::clone(&item_assets.texture_atlas_layout),
                    Item::BombMedium,
                    select_item::<1>
                ),
                // widget::item_button(
                //     Handle::clone(&item_assets.sprite_sheet),
                //     &ui_assets,
                //     Handle::clone(&item_assets.texture_atlas_layout),
                //     Item::BombLarge,
                //     select_item::<2>
                // ),
                widget::item_button(
                    Handle::clone(&item_assets.sprite_sheet),
                    &ui_assets,
                    Handle::clone(&item_assets.texture_atlas_layout),
                    Item::BombHorizontal,
                    select_item::<3>
                ),
                widget::item_button(
                    Handle::clone(&item_assets.sprite_sheet),
                    &ui_assets,
                    Handle::clone(&item_assets.texture_atlas_layout),
                    Item::BombVertical,
                    select_item::<4>
                ),
                widget::item_button(
                    Handle::clone(&item_assets.sprite_sheet),
                    &ui_assets,
                    Handle::clone(&item_assets.texture_atlas_layout),
                    Item::Eraser,
                    select_item::<255> // Eraser
                ),
            ],
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            align_items: AlignItems::FlexEnd,
            justify_content: JustifyContent::Center,
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(16.0),
            left: Val::Percent(80.0),
            ..Default::default()
        });
}

fn spawn_controlflow_buttons(mut commands: Commands, ui_assets: Res<UiAssets>) {
    commands
        .spawn((
            widget::ui_root("Control Flow Buttons"),
            GlobalZIndex(0),
            LevelBase,
            StateScoped(Screen::Gameplay),
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexEnd,
            height: Val::Percent(80.0),
            top: Val::Percent(10.0),
            left: Val::Percent(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(16.0),
            ..Default::default()
        })
        .with_children(|parent| {
            // parent.spawn(widget::menu_button(&ui_assets));
            parent.spawn(widget::run_button(&ui_assets, run_simulation_with_button));
        });
}

fn init_edit_state(mut selected_item: ResMut<SelectedItem>) {
    selected_item.0 = None; // Reset selected item
}

#[derive(Debug, Clone, Event)]
pub struct CreateObject {
    pub parent_grid: Entity,
    pub coord: GridCoord,
    pub item: Item,
    with_sound: bool,
}

impl CreateObject {
    pub fn new(parent_grid: Entity, coord: GridCoord, item: Item) -> Self {
        Self {
            parent_grid,
            coord,
            item,
            with_sound: true,
        }
    }

    pub fn without_sound(mut self) -> Self {
        self.with_sound = false;
        self
    }

    pub fn with_sound(mut self) -> Self {
        self.with_sound = true;
        self
    }
}

#[allow(dead_code)]
#[derive(Event, Debug, Clone)]
pub struct CreateFire {
    pub _parent_grid: Entity,
    pub coord: GridCoord,
}

#[derive(Component, Debug)]
#[require(FireAnimation)]
pub struct Fire;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub(super) struct SelectedItem(pub Option<Item>);

fn select_item<const I: u8>(
    _: Trigger<Pointer<Click>>,
    mut selected_item: ResMut<SelectedItem>,
    game_phase: Res<State<GamePhase>>,
) {
    let item = Item::from(I);
    selected_item.0 = if selected_item.0 == Some(item) || *game_phase.get() != GamePhase::Edit {
        None
    } else {
        Some(item)
    }
}

fn highlight_selected_item(
    selected_item: Res<SelectedItem>,
    query: Query<(&mut ImageNode, &Item), With<widget::ItemButton>>,
) {
    for (mut image_node, &item) in query {
        image_node
            .texture_atlas
            .iter_mut()
            .for_each(|texture_atlas| {
                texture_atlas.index = if selected_item.0 == Some(item) {
                    0 // Highlighted state
                } else {
                    1 // Normal state
                };
            });
    }
}

// create item on grid click
fn create_object(
    trigger: Trigger<CreateObject>,
    mut commands: Commands,
    item_assets: Res<ItemAssets>,
    query: Query<(Entity, &Item, &GridCoord)>,
    se_assets: Option<Res<SoundEffectAssets>>,
    se_volume: Res<SEVolume>,
) {
    let event = trigger.event();

    if let Some((existing_entity, _item, _coord)) =
        query.iter().find(|&(_, _, coord)| coord == &event.coord)
    {
        commands.entity(existing_entity).despawn();
    }

    if event.item == Item::Eraser {
        if event.with_sound {
            if let Some(se_assets) = se_assets {
                commands.spawn(sound_effect(se_assets.break_1.clone(), &se_volume));
            }
        }
        return;
    }

    let entity = commands
        .spawn((
            Name::new("Item Object"),
            GridCoord::clone(&event.coord),
            Item::clone(&event.item),
            ItemState::None,
            Sprite::from_atlas_image(
                item_assets.sprite_sheet.clone(),
                TextureAtlas {
                    layout: item_assets.texture_atlas_layout.clone(),
                    index: event.item as usize,
                },
            ),
            Transform::from_scale(Vec3::splat(2.0)).with_translation(Vec3::new(0.0, 0.0, 1.0)),
            StateScoped(Screen::Gameplay),
        ))
        .id();

    commands.entity(event.parent_grid).add_child(entity);

    if event.with_sound {
        if let Some(se_assets) = se_assets {
            commands.spawn(sound_effect(se_assets.break_2.clone(), &se_volume));
        }
    }
}

fn _try_create_single_fire(
    trigger: Trigger<CreateFire>,
    mut commands: Commands,
    item_query: Query<(Entity, &Item, &GridCoord), Without<Fire>>,
    fire_query: Query<(Entity, &GridCoord), With<Fire>>,
    item_assets: Res<ItemAssets>,
) {
    // if there is no bomb at the coordinate, do nothing
    let Some((parent_entity, _item, _coord)) = item_query
        .iter()
        .find(|&(_, item, coord)| item.is_bomb() && *coord == trigger.coord)
    else {
        return;
    };

    if let Ok((fire_entity, &fire_coord)) = fire_query.single() {
        commands.entity(fire_entity).despawn();
        if fire_coord == trigger.coord {
            return;
        }
    }

    commands
        .entity(parent_entity)
        .with_child(fire(trigger.coord, &item_assets));
}

pub fn fire(coord: GridCoord, item_assets: &ItemAssets) -> impl Bundle {
    (
        Name::new("Fire Object"),
        coord,
        Fire,
        Sprite::from_atlas_image(
            item_assets.sprite_sheet.clone(),
            TextureAtlas {
                layout: item_assets.texture_atlas_layout.clone(),
                index: 5,
            },
        ),
        Transform::from_translation(Vec3::new(2.0, 2.0, 0.1)),
        StateScoped(Screen::Gameplay),
    )
}

fn reset_all_object_placements(
    button_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    if button_input.just_pressed(KeyCode::KeyR) {
        next_state.set(GamePhase::Init);
    }
}

fn run_simulation_with_keyboard(
    button_input: Res<ButtonInput<KeyCode>>,
    fire_query: Query<Entity, With<Fire>>,
    next_state: ResMut<NextState<GamePhase>>,
    mut commands: Commands,
    se_assets: Option<Res<SoundEffectAssets>>,
    se_volume: Res<SEVolume>,
) {
    if button_input.just_pressed(KeyCode::Space) {
        if let Some(se_assets) = se_assets {
            commands.spawn(sound_effect(se_assets.start_1.clone(), &se_volume));
        }
        _try_run_simulation(fire_query, next_state);
    }
}

fn run_simulation_with_button(
    _trigger: Trigger<Pointer<Click>>,
    state: Res<State<GamePhase>>,
    fire_query: Query<Entity, With<Fire>>,
    next_state: ResMut<NextState<GamePhase>>,
) {
    if *state.get() != GamePhase::Edit {
        return;
    }

    _try_run_simulation(fire_query, next_state);
}

fn _try_run_simulation(
    fire_query: Query<Entity, With<Fire>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    if fire_query.is_empty() {
    } else {
        next_state.set(GamePhase::Run);
    }
}

#[derive(Resource, Reflect, Debug, Default)]
#[reflect(Resource)]
pub struct CurrentPlacement {
    level: usize,
    placements: Vec<(GridCoord, Item)>,
}

impl CurrentPlacement {
    pub fn new(level: usize, placements: Vec<(GridCoord, Item)>) -> Self {
        Self { level, placements }
    }
}

fn apply_current_placement(
    mut commands: Commands,
    current_placement: Res<CurrentPlacement>,
    current_level: Res<CurrentLevel>,
    grid_query: Query<(&GridCoord, Entity), With<GridTile>>,
) {
    if current_placement.level != current_level.level {
        return; // Do not apply if the level has changed
    }

    for &(coord, item) in &current_placement.placements {
        if let Some((_, parent_grid)) = grid_query
            .iter()
            .find(|&(&grid_coord, _)| grid_coord == coord)
        {
            commands.trigger(CreateObject::new(parent_grid, coord, item).without_sound());
        } else {
            warn!("No grid tile found for coord: {:?}", coord);
            continue;
        }
    }
}

fn reset_current_placement(mut current_placement: ResMut<CurrentPlacement>) {
    current_placement.placements.clear(); // Clear the current placement
    current_placement.level = usize::MAX; // Reset the level to an invalid state
}
