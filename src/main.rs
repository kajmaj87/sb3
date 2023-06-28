mod business;
mod config;
mod money;
mod people;
mod user_input;

use bevy::log::LogPlugin;
use std::any::TypeId;
use std::collections::{BTreeMap, HashMap};

use crate::business::SellOrder;
use crate::config::Config;
use bevy::prelude::*;
use bevy_asset::{HandleId, ReflectAsset};
use bevy_egui::EguiContext;
use bevy_egui::EguiSet;
use bevy_inspector_egui::bevy_inspector::hierarchy::{hierarchy_ui, SelectedEntities};
use bevy_inspector_egui::bevy_inspector::{
    self, ui_for_entities_shared_components, ui_for_entity_with_children,
};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_reflect::TypeRegistry;
use bevy_render::camera::Viewport;
use bevy_window::PrimaryWindow;
use egui::plot::{Bar, BarChart, Corner, Legend, Plot};
use egui::Color32;
use egui_dock::{NodeIndex, Tree};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "info,wgpu_core=warn,wgpu_hal=warn,sb3=debug".into(),
            level: bevy::log::Level::WARN,
        }))
        .add_plugin(DefaultInspectorConfigPlugin)
        .add_plugin(bevy_egui::EguiPlugin)
        .add_plugin(config::ConfigPlugin)
        // .add_plugins(
        //     DefaultPlugins
        //         .set(ImagePlugin::default_nearest())
        //         .set(LogPlugin {
        //             filter: "info,wgpu_core=warn,wgpu_hal=warn,sb3=warn".into(),
        //             level: bevy::log::Level::WARN,
        //         }),
        // )
        // .add_plugins(bevy_mod_picking::plugins::DefaultPickingPlugins)
        .insert_resource(UiState::new())
        .add_startup_system(setup)
        .add_system(
            show_ui_system
                .in_base_set(CoreSet::PostUpdate)
                .before(EguiSet::ProcessOutput)
                .before(bevy::transform::TransformSystem::TransformPropagate),
        )
        .add_system(
            set_camera_viewport
                .in_base_set(CoreSet::PostUpdate)
                .after(show_ui_system),
        )
        .insert_resource(Days {
            days: 0,
            next_turn: false,
            last_update: 0.0,
        })
        .insert_resource(Counter(0))
        .add_system(user_input::input_system.in_base_set(CoreSet::First))
        .add_system(
            date_update_system
                .run_if(should_advance_day)
                .in_base_set(CoreSet::PreUpdate),
        )
        .add_system(count_system.run_if(next_turn))
        .add_system(business::produce.run_if(next_turn))
        .add_system(business::create_sell_orders.run_if(next_turn))
        .add_system(business::update_sell_order_prices.run_if(next_turn))
        .add_system(business::create_buy_orders.run_if(next_turn))
        .add_system(business::execute_orders_for_manufacturers.run_if(next_turn))
        .add_system(turn_end_system.in_base_set(CoreSet::PostUpdate))
        .add_startup_system(business::init)
        .register_type::<Option<Handle<Image>>>()
        .register_type::<AlphaMode>()
        .run();
}

#[derive(Component)]
struct MainCamera;

#[derive(Resource)]
pub struct Days {
    days: usize,
    next_turn: bool,
    last_update: f32,
}

impl Days {
    fn next_day(&mut self, time: Res<Time>) {
        self.days += 1;
        self.next_turn = true;
        self.last_update = time.elapsed_seconds();
    }
}

#[derive(Resource)]
struct Counter(usize);

fn date_update_system(mut days: ResMut<Days>, time: Res<Time>) {
    days.next_day(time);
}

fn count_system(mut counter: ResMut<Counter>) {
    counter.0 += 1;
}

fn should_advance_day(time: Res<Time>, days: Res<Days>, config: Res<Config>) -> bool {
    if config.game.speed.value == 0.0 {
        return false;
    }
    time.elapsed_seconds() - days.last_update > config.game.speed.value
}

fn turn_end_system(mut days: ResMut<Days>) {
    days.next_turn = false;
}

fn next_turn(days: Res<Days>) -> bool {
    days.next_turn
}

fn show_ui_system(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world) else { return; };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<UiState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut())
    });
}

// make camera only render to view not obstructed by UI
fn set_camera_viewport(
    ui_state: Res<UiState>,
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    egui_settings: Res<bevy_egui::EguiSettings>,
    mut cameras: Query<&mut Camera, With<MainCamera>>,
) {
    let mut cam = cameras.single_mut();

    let Ok(window) = primary_window.get_single() else { return; };

    let scale_factor = window.scale_factor() * egui_settings.scale_factor;

    let viewport_pos = ui_state.viewport_rect.left_top().to_vec2() * scale_factor as f32;
    let viewport_size = ui_state.viewport_rect.size() * scale_factor as f32;

    cam.viewport = Some(Viewport {
        physical_position: UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32),
        physical_size: UVec2::new(viewport_size.x as u32, viewport_size.y as u32),
        depth: 0.0..1.0,
    });
}

#[derive(Eq, PartialEq)]
enum InspectorSelection {
    Entities,
    Resource(TypeId, String),
    Asset(TypeId, String, HandleId),
}

#[derive(Resource)]
struct UiState {
    tree: Tree<EguiWindow>,
    viewport_rect: egui::Rect,
    selected_entities: SelectedEntities,
    selection: InspectorSelection,
}

impl UiState {
    pub fn new() -> Self {
        let mut tree = Tree::new(vec![EguiWindow::GameView]);
        let [game, _inspector] =
            tree.split_right(NodeIndex::root(), 0.75, vec![EguiWindow::Inspector]);
        let [game, _hierarchy] = tree.split_left(game, 0.2, vec![EguiWindow::Hierarchy]);
        let [_game, _bottom] = tree.split_below(
            game,
            0.8,
            vec![EguiWindow::Days, EguiWindow::Resources, EguiWindow::Assets],
        );

        Self {
            tree,
            selected_entities: SelectedEntities::default(),
            selection: InspectorSelection::Entities,
            viewport_rect: egui::Rect::NOTHING,
        }
    }

    fn ui(&mut self, world: &mut World, ctx: &mut egui::Context) {
        let mut tab_viewer = TabViewer {
            world,
            viewport_rect: &mut self.viewport_rect,
            selected_entities: &mut self.selected_entities,
            selection: &mut self.selection,
        };
        egui_dock::DockArea::new(&mut self.tree)
            .style(egui_dock::Style {
                tab_bar_background_color: ctx.style().visuals.window_fill(),
                ..egui_dock::Style::from_egui(ctx.style().as_ref())
            })
            .show(ctx, &mut tab_viewer);
    }
}

#[derive(Debug)]
enum EguiWindow {
    GameView,
    Hierarchy,
    Resources,
    Assets,
    Inspector,
    Days,
}

struct TabViewer<'a> {
    world: &'a mut World,
    selected_entities: &'a mut SelectedEntities,
    selection: &'a mut InspectorSelection,
    viewport_rect: &'a mut egui::Rect,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = EguiWindow;

    fn ui(&mut self, ui: &mut egui::Ui, window: &mut Self::Tab) {
        let type_registry = self.world.resource::<AppTypeRegistry>().0.clone();
        let type_registry = type_registry.read();

        match window {
            EguiWindow::Days => {
                ui.label(format!("Days: {}", self.world.resource::<Days>().days));
                ui.label(format!("Count: {}", self.world.resource::<Counter>().0));
                let mut grouped_orders = BTreeMap::new();

                for sell_order in self.world.query::<&SellOrder>().iter(self.world) {
                    grouped_orders
                        .entry(sell_order.item_type.clone())
                        .or_insert_with(Vec::new)
                        .push(sell_order.price);
                }

                for (item_type, prices) in grouped_orders {
                    let len = prices.len();
                    let mut prices = prices;
                    prices.sort_unstable();

                    let min = *prices.first().unwrap();
                    let max = *prices.last().unwrap();
                    let median = prices[len / 2];
                    let p10 = prices[(len as f32 * 0.1).round() as usize];
                    let p90 = prices[(len as f32 * 0.9).round() as usize];

                    ui.label(format!(
                        "ItemType: {}\nMin: {} p10: {} Median: {} p90: {} Max: {}, total: {}, avg. {}",
                        item_type.name, min, p10, median, p90, max, len, prices.iter().sum::<u64>() / len as u64
                    ));
                    Plot::new(item_type.name.clone())
                        .view_aspect(2.0)
                        .legend(Legend {
                            position: Corner::LeftTop,
                            ..default()
                        })
                        .show(ui, |plot_ui| {
                            plot_ui.bar_chart(create_histogram(
                                format!("{} prices", item_type.name).as_str(),
                                &prices,
                                20,
                            ));
                        });
                }
                // let mut prices: Vec<u64> = self.world.query::<(Entity, &SellOrder)>()
                //     .iter(&self.world)
                //     .map(|(_, sell_order)| sell_order.base_price)
                //     .collect();
                //
                // prices.sort_unstable();
                //
                // let len = prices.len();
                // if len > 0 {
                //     let min = *prices.first().unwrap_or(&0);
                //     let max = *prices.last().unwrap_or(&0);
                //     let median = prices[len / 2];
                //     let p10 = prices[len / 10];
                //     let p90 = prices[9 * len / 10];
                //     ui.label(format!("Min: {} p10: {} Median: {} p90: {} Max: {}, total orders: {}", min, p10, median, p90, max, len));
                // }
            }
            EguiWindow::GameView => {
                *self.viewport_rect = ui.clip_rect();
            }
            EguiWindow::Hierarchy => {
                let selected = hierarchy_ui(self.world, ui, self.selected_entities);
                if selected {
                    *self.selection = InspectorSelection::Entities;
                }
            }
            EguiWindow::Resources => select_resource(ui, &type_registry, self.selection),
            EguiWindow::Assets => select_asset(ui, &type_registry, self.world, self.selection),
            EguiWindow::Inspector => match *self.selection {
                InspectorSelection::Entities => match self.selected_entities.as_slice() {
                    &[entity] => ui_for_entity_with_children(self.world, entity, ui),
                    entities => ui_for_entities_shared_components(self.world, entities, ui),
                },
                InspectorSelection::Resource(type_id, ref name) => {
                    ui.label(name);
                    bevy_inspector::by_type_id::ui_for_resource(
                        self.world,
                        type_id,
                        ui,
                        name,
                        &type_registry,
                    )
                }
                InspectorSelection::Asset(type_id, ref name, handle) => {
                    ui.label(name);
                    bevy_inspector::by_type_id::ui_for_asset(
                        self.world,
                        type_id,
                        handle,
                        ui,
                        &type_registry,
                    );
                }
            },
        }
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui::WidgetText {
        format!("{window:?}").into()
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, EguiWindow::GameView)
    }
}

pub fn create_histogram(name: &str, values: &[u64], bins: u32) -> BarChart {
    let mut histogram = HashMap::new();
    let max = values.iter().max().unwrap_or(&0);
    let min = values.iter().min().unwrap_or(&0);
    let range = max - min + 1;
    let bin_width = (range as f64 / bins as f64).ceil() as u64;
    for &value in values {
        *histogram.entry((value - min) / bin_width).or_insert(0) += 1;
    }
    let histogram: Vec<Bar> = histogram
        .into_iter()
        .map(|(bin, count)| {
            Bar::new((bin * bin_width + min) as f64, count as f64).width(bin_width as f64)
        })
        .collect();
    BarChart::new(histogram)
        .color(Color32::LIGHT_BLUE)
        .name(name)
}

fn select_resource(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    selection: &mut InspectorSelection,
) {
    let mut resources: Vec<_> = type_registry
        .iter()
        .filter(|registration| registration.data::<ReflectResource>().is_some())
        .map(|registration| (registration.short_name().to_owned(), registration.type_id()))
        .collect();
    resources.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

    for (resource_name, type_id) in resources {
        let selected = match *selection {
            InspectorSelection::Resource(selected, _) => selected == type_id,
            _ => false,
        };

        if ui.selectable_label(selected, &resource_name).clicked() {
            *selection = InspectorSelection::Resource(type_id, resource_name);
        }
    }
}

fn select_asset(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    world: &World,
    selection: &mut InspectorSelection,
) {
    let mut assets: Vec<_> = type_registry
        .iter()
        .filter_map(|registration| {
            let reflect_asset = registration.data::<ReflectAsset>()?;
            Some((
                registration.short_name().to_owned(),
                registration.type_id(),
                reflect_asset,
            ))
        })
        .collect();
    assets.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));

    for (asset_name, asset_type_id, reflect_asset) in assets {
        let mut handles: Vec<_> = reflect_asset.ids(world).collect();
        handles.sort();

        ui.collapsing(format!("{asset_name} ({})", handles.len()), |ui| {
            for handle in handles {
                let selected = match *selection {
                    InspectorSelection::Asset(_, _, selected_id) => selected_id == handle,
                    _ => false,
                };

                if ui
                    .selectable_label(selected, format!("{:?}", handle))
                    .clicked()
                {
                    *selection =
                        InspectorSelection::Asset(asset_type_id, asset_name.clone(), handle);
                }
            }
        });
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let box_size = 2.0;
    let box_thickness = 0.15;
    let box_offset = (box_size + box_thickness) / 2.0;

    // left - red
    let mut transform = Transform::from_xyz(-box_offset, box_offset, 0.0);
    transform.rotate(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size,
            box_thickness,
            box_size,
        ))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.63, 0.065, 0.05),
            ..Default::default()
        }),
        ..Default::default()
    });
    // right - green
    let mut transform = Transform::from_xyz(box_offset, box_offset, 0.0);
    transform.rotate(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size,
            box_thickness,
            box_size,
        ))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.14, 0.45, 0.091),
            ..Default::default()
        }),
        ..Default::default()
    });
    // bottom - white
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size + 2.0 * box_thickness,
            box_thickness,
            box_size,
        ))),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.725, 0.71, 0.68),
            ..Default::default()
        }),
        ..Default::default()
    });
    // top - white
    let transform = Transform::from_xyz(0.0, 2.0 * box_offset, 0.0);
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size + 2.0 * box_thickness,
            box_thickness,
            box_size,
        ))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.725, 0.71, 0.68),
            ..Default::default()
        }),
        ..Default::default()
    });
    // back - white
    let mut transform = Transform::from_xyz(0.0, box_offset, -box_offset);
    transform.rotate(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(
            box_size + 2.0 * box_thickness,
            box_thickness,
            box_size + 2.0 * box_thickness,
        ))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.725, 0.71, 0.68),
            ..Default::default()
        }),
        ..Default::default()
    });

    // ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.02,
    });
    // top light
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane::from_size(0.4))),
            transform: Transform::from_matrix(Mat4::from_scale_rotation_translation(
                Vec3::ONE,
                Quat::from_rotation_x(std::f32::consts::PI),
                Vec3::new(0.0, box_size + 0.5 * box_thickness, 0.0),
            )),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE * 100.0,
                ..Default::default()
            }),
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn(PointLightBundle {
                point_light: PointLight {
                    color: Color::WHITE,
                    intensity: 25.0,
                    ..Default::default()
                },
                transform: Transform::from_translation((box_thickness + 0.05) * Vec3::Y),
                ..Default::default()
            });
        });
    // directional light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 2.0)),
        ..Default::default()
    });

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, box_offset, 4.0)
                .looking_at(Vec3::new(0.0, box_offset, 0.0), Vec3::Y),
            ..Default::default()
        },
        MainCamera,
        // PickRaycastSource,
    ));
}
