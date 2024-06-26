#[macro_use]
extern crate itertools;
#[macro_use]
extern crate worldgen;

mod component;
mod game;
mod gui;
mod managers;
mod structures;
mod util;

use std::error::Error;
use std::io;
use std::time::{Duration, SystemTime};

use tui::backend::CrosstermBackend;
use tui::layout::Margin;
use tui::widgets::Paragraph;
use tui::Terminal;

use crossterm::event::{poll, read, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use worldgen::world::Size;

use crate::game::{Commodity, Manufactured, MapController, Resource};
use crate::gui::{
    FactoryCommoditySelect, Menu, MenuSelector, MineResourceSelect, RefineryResourceSelect,
};
use crate::managers::{EnergyManager, ResourceManager};
use crate::structures::{StructureFactory, StructureGroup};

use crate::util::format_welcome_message;
use crate::util::{EventBus, GameEvent, Tick};

fn main() -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;
    enable_raw_mode()?;

    let now = SystemTime::now();
    let events = EventBus::new();

    let mut log_buffer = String::default();
    log_buffer.push_str(&format_welcome_message());

    // For keeping game update interval
    let mut update_tick = Tick::new();

    // For keeping game draw interval
    let mut draw_tick = Tick::new();

    let mut energy_manager = EnergyManager::new();

    // An object for player score keeping and updating.
    let storage_resources = vec![
        Resource::Iron,
        Resource::Aluminum,
        Resource::Carbon,
        Resource::Silica,
        Resource::Uranium,
        Resource::Water,
    ];

    let storage_manufactured = vec![
        Manufactured::Silicon,
        Manufactured::Food,
        Manufactured::Steel,
        Manufactured::BioPlastic,
        Manufactured::Oxygen,
        Manufactured::Gravel,
        Manufactured::Hydrogen,
        Manufactured::FuelPellet,
    ];

    let storage_commodities = vec![
        Commodity::Concrete,
        Commodity::Semiconductor,
        Commodity::Fuel,
        Commodity::Glass,
        Commodity::FuelRod,
    ];

    let mut resource_manager =
        ResourceManager::new(storage_resources, storage_manufactured, storage_commodities);

    let mut menu = Menu::new(vec![
        StructureGroup::Base,
        StructureGroup::Power,
        StructureGroup::Mine,
        StructureGroup::Refinery,
        StructureGroup::Factory,
        StructureGroup::Storage,
    ]);

    let mut mine_select = MineResourceSelect::new(vec![
        Resource::Iron,
        Resource::Aluminum,
        Resource::Carbon,
        Resource::Silica,
        Resource::Uranium,
        Resource::Water,
    ]);

    let mut refinery_select = RefineryResourceSelect::new(vec![
        vec![Manufactured::Silicon],
        vec![Manufactured::Food],
        vec![Manufactured::Steel],
        vec![Manufactured::BioPlastic],
        vec![Manufactured::Hydrogen, Manufactured::Oxygen],
        vec![Manufactured::FuelPellet],
    ]);

    let mut factory_select = FactoryCommoditySelect::new(vec![
        Commodity::Concrete,
        Commodity::Semiconductor,
        Commodity::Fuel,
        Commodity::Glass,
        Commodity::FuelRod,
    ]);

    // The game controller, work with the Map object.
    let mut controller = MapController::new(Size::of(90, 40));

    // Default margin used when drawing interfaces.
    let margin_1 = Margin {
        vertical: 1,
        horizontal: 1,
    };

    let mut map_widget: Option<Paragraph> = None;

    controller.generate_deposits();

    loop {
        let elapsed = now.elapsed()?;
        let game_event = events.next()?;

        terminal.draw(|frame| {
            let main_layout = gui::build_main_layout(frame.size());
            let left_layout = gui::build_left_layout(main_layout[0]);
            let right_layout = gui::build_right_layout(main_layout[2]);
            let menu_layout = gui::build_menu_layout(right_layout[0]);
            let colony_layout = gui::build_colony_layout(left_layout[0]);

            let stats_widget_left = gui::draw_stats_widget_left(
                &resource_manager,
                &energy_manager,
                elapsed,
                update_tick.delta(),
                draw_tick.delta(),
            );

            let stats_widget_right = gui::draw_stats_widget_right(&resource_manager);

            frame.render_widget(stats_widget_left, colony_layout[0]);
            frame.render_widget(stats_widget_right, colony_layout[1]);

            let console_widget = gui::draw_console_widget(&log_buffer);
            frame.render_widget(console_widget, left_layout[1]);

            let build_menu = gui::draw_structure_menu_widget(&menu);
            frame.render_widget(build_menu, menu_layout[0]);

            match menu.selected() {
                StructureGroup::Base => {}
                StructureGroup::Power => {}
                StructureGroup::Mine => {
                    // let resource_select_widget = gui::draw_mine_select_widget(&mine_select);
                    // frame.render_widget(resource_select_widget, menu_layout[1]);
                }
                StructureGroup::Factory => {
                    let commodity_select_widget = gui::draw_factory_select_widget(&factory_select);
                    frame.render_widget(commodity_select_widget, menu_layout[1]);
                }
                StructureGroup::Refinery => {
                    let commodity_select_widget =
                        gui::draw_refinery_select_widget(&refinery_select);
                    frame.render_widget(commodity_select_widget, menu_layout[1]);
                }
                StructureGroup::Storage => {}
            }

            let info_panel = gui::draw_info_widget(
                controller.position(),
                controller.tile(),
                controller.object(),
            );
            frame.render_widget(info_panel, right_layout[1]);

            let map_block = gui::draw_map_block();
            frame.render_widget(map_block, main_layout[1]);

            let map_viewport = main_layout[1].inner(&margin_1);

            // If the widget was drawn by the draw event, render it, otherwise do not.
            if map_widget.is_some() {
                frame.render_widget(map_widget.clone().unwrap(), map_viewport);
            }
        })?;

        match game_event {
            // When we get the draw event, we'll update the game map.
            // Map will not be drawn every loop iteration.
            GameEvent::Update => {
                energy_manager.zero();

                energy_manager.collect(controller.objects_mut().list());

                let objects = controller.objects_mut().list_mut();
                resource_manager.collect(objects, &mut energy_manager);

                // if we discharged energy from storage, discharge batteries.
                if energy_manager.discharged() > 0 {
                    energy_manager.discharge(controller.objects_mut().list_mut());
                }

                // if we have available energy output, use it to charge batteries.
                if energy_manager.output() > 0 {
                    energy_manager.charge(controller.objects_mut().list_mut());
                }

                update_tick.update(&elapsed);
            }
            GameEvent::Draw => {
                let map_text = gui::render_map(
                    controller.map(),
                    controller.objects(),
                    controller.position(),
                );
                map_widget = Option::from(gui::draw_map_widget(&map_text));
                draw_tick.update(&elapsed);
            }
            GameEvent::Input => {
                if poll(Duration::from_millis(0))? {
                    // It's guaranteed that the `read()` won't block when the `poll()`
                    // function returns `true`
                    match read()? {
                        Event::Key(event) => {
                            // let log = util::get_log(format!("{:?}", event));
                            // log_buffer.push_str(&log);
                            match event.code {
                                KeyCode::Backspace => {}
                                KeyCode::Left => {
                                    controller.left();
                                }
                                KeyCode::Enter => {
                                    let structure_group = menu.selected();
                                    let tile = controller.tile();
                                    let object = controller.object();

                                    if StructureFactory::allowed(&structure_group, tile) {
                                        let structure = StructureFactory::new(
                                            &structure_group,
                                            object,
                                            &resource_manager,
                                            &refinery_select,
                                            &factory_select,
                                        );

                                        if structure.is_some() {
                                            controller.add_structure(structure.unwrap());
                                        }
                                    }
                                }
                                KeyCode::Right => {
                                    controller.right();
                                }
                                KeyCode::Up => {
                                    controller.up();
                                }
                                KeyCode::Down => {
                                    controller.down();
                                }
                                KeyCode::Home => match menu.selected() {
                                    StructureGroup::Base => {}
                                    StructureGroup::Power => {}
                                    StructureGroup::Mine => {
                                        mine_select.previous();
                                    }
                                    StructureGroup::Refinery => {
                                        refinery_select.previous();
                                    }
                                    StructureGroup::Factory => {
                                        factory_select.previous();
                                    }
                                    StructureGroup::Storage => {}
                                },
                                KeyCode::End => match menu.selected() {
                                    StructureGroup::Base => {}
                                    StructureGroup::Power => {}
                                    StructureGroup::Mine => {
                                        mine_select.next();
                                    }
                                    StructureGroup::Refinery => {
                                        refinery_select.next();
                                    }
                                    StructureGroup::Factory => {
                                        factory_select.next();
                                    }
                                    StructureGroup::Storage => {}
                                },
                                KeyCode::PageUp => {
                                    menu.previous();
                                }
                                KeyCode::PageDown => {
                                    menu.next();
                                }
                                KeyCode::Tab => {}
                                KeyCode::BackTab => {}
                                KeyCode::Delete => {
                                    controller.destroy_structure();
                                }
                                KeyCode::Insert => {}
                                KeyCode::F(_) => {}
                                KeyCode::Char(c) => match c {
                                    'a' => {
                                        controller.left();
                                    }
                                    'd' => {
                                        controller.right();
                                    }
                                    'w' => {
                                        controller.up();
                                    }
                                    's' => {
                                        controller.down();
                                    }
                                    _ => {}
                                },
                                KeyCode::Null => {}
                                KeyCode::Esc => {
                                    // Quit
                                    disable_raw_mode()?;
                                    terminal.clear()?;
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Event::Mouse(event) => {
                            let log = util::get_log(format!("{:?}", event));
                            log_buffer.push_str(&log);
                        }
                        Event::Resize(width, height) => {
                            let message = format!("Screen Resize ({}x{})", width, height);
                            let log = util::get_log(message);
                            log_buffer.push_str(&log);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}
