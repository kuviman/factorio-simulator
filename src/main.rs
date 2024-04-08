use data::FuelCategory;
use number::Number;
use smart::Tasks;

mod data;
mod number;
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

    let mut world = smart::World::new(data::RecipeMode::Normal)?;

    let mut tasks = Tasks::default();
    for line in std::io::stdin().lines() {
        let line = line.expect("Failed to read line");
        if line.starts_with('#') {
            // comment
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(command) = parts.next() else {
            continue;
        };
        match command {
            "prefer-fuel" => {
                let category: FuelCategory =
                    serde_json::from_str(&format!("{:?}", parts.next().unwrap())).unwrap();
                let item = parts.next().unwrap();
                world.prefer_fuel(category, item);
            }
            "build" => {
                let machine = parts.next().unwrap();
                let amount: Number = parts.next().unwrap_or("1").parse().unwrap();
                world.build(machine, amount);
            }
            "craft" => {
                let item = parts.next().unwrap();
                let amount: Number = parts.next().unwrap_or("1").parse().unwrap();
                world.craft(item, amount);
            }
            "flush" => {
                world
                    .planner()
                    .add_tasks(std::mem::take(&mut tasks))
                    .think()
                    .execute(&mut world);
            }
            "research" => {
                world
                    .planner()
                    .add_tasks(std::mem::take(&mut tasks))
                    .think()
                    .execute(&mut world);
                let research = parts.next().unwrap();
                world.research(research);
            }
            "destroy-all" => {
                let machine = parts.next().unwrap();
                world.destroy_all(machine);
            }
            _ => panic!("unknown command {command:?}"),
        }
    }
    Ok(())
}
