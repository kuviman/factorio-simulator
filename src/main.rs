use number::Number;
use raw_data::FuelCategory;
use smart::Tasks;

mod data;
mod number;
mod raw_data;
mod smart;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::new()
        .filter_level(if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .parse_default_env()
        .init();

    let mut world = smart::World::new(raw_data::RecipeMode::Normal, 1.into())?;
    let mut current_tasks: Option<Tasks> = None;

    for line in std::io::stdin().lines() {
        let line = line.expect("Failed to read line");
        let line = line.trim();
        if line.starts_with('#') {
            // comment
            // ^ good comment
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(command) = parts.next() else {
            continue;
        };
        match command {
            "{" => {
                current_tasks = Some(Tasks::default());
            }
            "}" => {
                let tasks = current_tasks.take().expect("} after no { ???");
                world.planner().add_tasks(tasks).think().execute(&mut world);
            }
            "prefer-fuel" => {
                assert!(current_tasks.is_none());
                let category: FuelCategory =
                    serde_json::from_str(&format!("{:?}", parts.next().unwrap())).unwrap();
                let item = parts.next().unwrap();
                world.prefer_fuel(category, item);
            }
            "place" => {
                assert!(current_tasks.is_none());
                let machine = parts.next().unwrap();
                let amount: Number = parts.next().unwrap_or("1").parse().unwrap();
                *world.machines.entry(machine.into()).or_default() += amount;
            }
            "build" => {
                let machine = parts.next().unwrap();
                let amount: Number = parts.next().unwrap_or("1").parse().unwrap();
                if let Some(tasks) = &mut current_tasks {
                    *tasks.build.entry(machine.into()).or_default() += amount;
                } else {
                    world.build(machine, amount);
                }
            }
            "craft" => {
                let item = parts.next().unwrap();
                let amount: Number = parts.next().unwrap_or("1").parse().unwrap();
                if let Some(tasks) = &mut current_tasks {
                    *tasks.craft.entry(item.into()).or_default() += amount;
                } else {
                    world.craft(item, amount);
                }
            }
            "research" => {
                assert!(current_tasks.is_none());
                let research = parts.next().unwrap();
                world.research(research);
            }
            "unresearch" => {
                assert!(current_tasks.is_none());
                let research = parts.next().unwrap();
                world.unresearch(research);
            }
            "reset-counts" => {
                world.reset_counts();
            }
            "destroy-all" => {
                assert!(current_tasks.is_none());
                let machine = parts.next().unwrap();
                world.destroy_all(machine);
            }
            "show-counts" => {
                log::info!("Total crafts:");
                let mut total_crafts: Vec<_> = world.total_crafts.iter().collect();
                total_crafts.sort_by_key(|&(_, &amount)| amount);
                for (craft, amount) in total_crafts {
                    log::info!("{craft:?} = {amount:?}");
                }
            }
            _ => panic!("unknown command {command:?}"),
        }
    }
    Ok(())
}
