use crate::data::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use itertools::Itertools;

use crate::{
    number::Number,
    raw_data::{EnergyType, FuelCategory, RecipeMode, Seconds},
};

#[derive(Clone)]
pub struct World {
    data: Arc<Data>,
    pub no_thinking: bool,
    researches: HashSet<Arc<str>>,
    preferred_fuel: HashMap<FuelCategory, Item>,
    pub machines: HashMap<Arc<str>, Number>,
    time: Number<Seconds>,
    pub total_crafts: HashMap<Arc<str>, Number>,
    total_machine_time: Number<Seconds>,
}

#[derive(Debug, Default, Clone)]
struct ExecutedStep {
    crafts: HashMap<Arc<str>, Number>,
    builds: HashMap<Arc<str>, Number>,
    single_machine_time: HashMap<Arc<str>, Number<Seconds>>,
}

#[derive(Clone, Debug, Default)]
pub struct Plan {
    splits: Vec<Tasks>,
}

impl World {
    pub fn new(mode: RecipeMode, science_multiplier: Number) -> anyhow::Result<Self> {
        let data = Data::new(mode, science_multiplier)?;

        let mut machines = HashMap::new();
        machines.insert(CHARACTER_MINING.into(), 1.into());
        machines.insert(CHARACTER_CRAFTING.into(), 1.into());
        machines.insert(FREE_STUFF.into(), 1.into());

        Ok(Self {
            no_thinking: true,
            data: Arc::new(data),
            machines,
            preferred_fuel: HashMap::new(),
            researches: HashSet::new(),
            time: Number::new(0.0),
            total_crafts: HashMap::new(),
            total_machine_time: Number::new(0.0),
        })
    }

    pub fn craft(&mut self, item: impl Into<Item>, amount: impl Into<Number>) {
        self.planner()
            .add_tasks({
                let mut tasks = Tasks::default();
                tasks.craft.insert(item.into(), amount.into());
                tasks
            })
            .think()
            .execute(self);
    }

    pub fn build(&mut self, machine: impl Into<Item>, amount: impl Into<Number>) {
        self.planner()
            .add_tasks({
                let mut tasks = Tasks::default();
                tasks.build.insert(machine.into(), amount.into());
                tasks
            })
            .think()
            .execute(self);
    }

    pub fn prefer_fuel(&mut self, category: FuelCategory, item: impl Into<Item>) {
        self.preferred_fuel.insert(category, item.into());
    }

    pub fn destroy_all(&mut self, machine: impl Into<Item>) {
        let machine = machine.into();
        self.machines.remove(machine.name());
    }

    pub fn unresearch(&mut self, research: impl Into<Arc<str>>) {
        self.researches.remove(&research.into());
    }

    pub fn research(&mut self, research: impl Into<Arc<str>>) {
        let research = research.into();
        if self.researches.contains(&research) {
            return;
        }
        let data = self.data.clone();
        let research = &data.researches[&research];
        for dependency in &research.dependencies {
            self.research(dependency.clone());
        }

        self.planner()
            .add_tasks({
                let mut tasks = Tasks::default();
                tasks.craft_recipe.insert(
                    format!("research {:?}", research.name).as_str().into(),
                    1.into(),
                );
                tasks
            })
            .think()
            .execute(self);
        self.researches.insert(research.name.clone());
        log::info!("researched {:?}", research.name);
    }

    pub fn planner(&self) -> Planner<'_> {
        Planner {
            world: self,
            splits: Vec::new(),
        }
    }

    pub fn reset_counts(&mut self) {
        self.total_machine_time = Number::new(0.0);
        self.total_crafts.clear();
    }
}

pub struct Planner<'a> {
    world: &'a World,
    splits: Vec<Tasks>,
}

impl Planner<'_> {
    pub fn add_tasks(&mut self, tasks: Tasks) -> &mut Self {
        self.splits.push(tasks);
        self
    }
    pub fn think(&mut self) -> Plan {
        loop {
            if self.world.no_thinking {
                break;
            }
            let time = |world: &World| {
                (
                    Number::<Seconds>::new((world.time.value() / 60.0).round()),
                    world.total_machine_time,
                )
            };
            let time_to_beat = {
                let mut world = self.world.clone();
                for tasks in &self.splits {
                    tasks.execute(&mut world, false);
                }
                time(&world)
            };
            log::info!("Trying to improve {time_to_beat:?}");
            let mut improvement = None;
            for machine in self.world.machines.keys() {
                let machine = &**machine;
                for amount in [1] {
                    if find_recipe_for(self.world, machine).is_none() {
                        continue;
                    }
                    let mut improve_task = Tasks::default();
                    improve_task.build.insert(machine.into(), amount.into());
                    for pos in 0..=self.splits.len() {
                        // for pos in [0, self.splits.len()] {
                        let mut world = self.world.clone();
                        let mut new_splits = self.splits.clone();
                        new_splits.insert(pos, improve_task.clone());
                        for tasks in &new_splits {
                            tasks.execute(&mut world, false);
                        }
                        let time = time(&world);
                        if time < time_to_beat {
                            improvement = std::cmp::max_by_key(
                                improvement,
                                Some((time, new_splits)),
                                |imp| {
                                    std::cmp::Reverse(
                                        imp.as_ref().map_or(time_to_beat, |imp| imp.0),
                                    )
                                },
                            );
                        }
                    }
                }
            }
            if let Some((time, new_splits)) = improvement {
                self.splits = new_splits;
                log::trace!("improved time from {time_to_beat:?} to {time:?}");
            } else {
                break;
            }
        }
        Plan {
            splits: self.splits.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Tasks {
    pub build: HashMap<Item, Number>,
    pub craft: HashMap<Item, Number>,
    pub craft_recipe: HashMap<Arc<str>, Number>,
}

impl Tasks {
    fn execute(&self, world: &mut World, log: bool) {
        let mut planner = StepPlanner::new(world);
        for (item, amount) in &self.craft {
            planner.craft(item.clone(), *amount);
        }
        for (recipe, amount) in &self.craft_recipe {
            planner.craft_recipe(recipe.clone(), *amount);
        }
        for (machine, amount) in &self.build {
            planner.build(machine.clone(), *amount);
        }
        let planned = planner.finalize();
        if log {
            planned.log(world);
        }
        planned.execute(world);
        if log {
            log::info!("Time now is {:?}", world.time);
        }
    }
}

fn find_recipe_for(world: &World, item: impl Into<Item>) -> Option<Arc<str>> {
    let item = item.into();

    if let Item::Energy {
        fuel_category: Some(category),
        energy_type: EnergyType::Burner,
    } = item
    {
        let fuel_item = world
            .preferred_fuel
            .get(&category)
            .unwrap_or_else(|| panic!("No preferred fuel set for {category:?}"));
        // TODO this format is copypasta
        return Some(format!("{:?} {:?} burnable fuel energy", fuel_item.name(), category).into());
    }

    // TODO cache maybe?
    world
        .data
        .recipes
        .values()
        .filter(|recipe| recipe.results.contains_key(&item))
        .filter(|recipe| {
            world.machines.keys().any(|machine| {
                world.data.machines[machine]
                    .categories
                    .contains(&recipe.category)
            })
        })
        // mine rocks for stone, but not for coal
        .filter(|recipe| !(item.is("coal") && recipe.name.contains("pickaxe")))
        .filter(|recipe| !recipe.name.contains("barrel"))
        .filter(|recipe| &*recipe.name != "coal-liquefaction")
        .max_by_key(|recipe| {
            (
                matches!(recipe.category, Category::Free),
                &*recipe.name == "advanced-oil-processing",
            )
        })
        .map(|recipe| recipe.name.clone())
}

struct StepPlanner<'a> {
    world: &'a World,
    executed: ExecutedStep,
}

impl<'a> StepPlanner<'a> {
    fn finalize(mut self) -> ExecutedStep {
        let data = self.world.data.clone();
        let mut done = false;
        let mut total_times = HashMap::<Arc<str>, Number<Seconds>>::new();
        while !done {
            done = true;
            for (machine_name, single_machine_time) in
                std::mem::take(&mut self.executed.single_machine_time)
            {
                *total_times.entry(machine_name.clone()).or_default() += single_machine_time;
                let machine = &data.machines[&machine_name];
                for (energy_item, &usage) in &machine.energy_usage {
                    let energy_amount = usage * single_machine_time.convert::<()>();
                    if energy_amount.value() < 1.0 {
                        continue;
                    }
                    done = false;
                    self.craft(energy_item.clone(), energy_amount);
                }
            }
        }
        self.executed.single_machine_time = total_times;
        self.executed
    }
    fn build(&mut self, machine: Item, amount: Number) {
        *self
            .executed
            .builds
            .entry(machine.name().clone())
            .or_default() += amount;
        self.craft(machine, amount);
    }
    fn craft(&mut self, item: Item, amount: Number) {
        let recipe = find_recipe_for(self.world, item.clone())
            .unwrap_or_else(|| panic!("Could not find recipe for {item:?}"));
        log::trace!("craft {item:?} ({amount:?}) using {recipe:#?}");

        let recipe = &self.world.data.recipes[&recipe];
        let crafts = amount / recipe.results[&item];
        // TODO: im ignoring byproducts

        self.craft_recipe(recipe.name.clone(), crafts);
    }
    fn craft_recipe(&mut self, recipe: Arc<str>, crafts: Number) {
        let data = self.world.data.clone();
        let recipe = &data
            .recipes
            .get(&recipe)
            .unwrap_or_else(|| panic!("recipe {recipe:?} not found"));
        *self.executed.crafts.entry(recipe.name.clone()).or_default() += crafts;

        for (ingredient, &ingredient_amount) in &recipe.ingredients {
            self.craft(ingredient.clone(), ingredient_amount * crafts);
        }

        if let Some(recipe_crafting_time) = recipe.crafting_time {
            let machines_used = self
                .world
                .machines
                .iter()
                .filter(|&(name, _)| data.machines[name].categories.contains(&recipe.category));

            let total_speed = machines_used.clone().fold(
                Number::from(0),
                |sum, (machine_name, &machine_count)| {
                    sum + data.machines[machine_name].crafting_speed * machine_count
                },
            );

            if total_speed.value() == 0.0 {
                panic!("No machines that can craft {recipe:?}");
            }

            for (machine_name, &machine_count) in machines_used {
                let crafts = crafts * data.machines[machine_name].crafting_speed * machine_count
                    / total_speed;

                let single_machine_time =
                    crafts * recipe_crafting_time / data.machines[machine_name].crafting_speed;
                *self
                    .executed
                    .single_machine_time
                    .entry(machine_name.clone())
                    .or_default() += single_machine_time.convert::<Seconds>();
            }
        }
    }

    fn new(world: &'a World) -> Self {
        Self {
            world,
            executed: ExecutedStep::default(),
        }
    }
}

impl Plan {
    pub fn execute(self, world: &mut World) {
        for tasks in self.splits {
            // log::info!("Executing {step:#?}");
            tasks.execute(world, true);
        }
    }
}

// Every time I see veldak's name I start salivating like Pavlov's dogs.
// veldak consumes only the most delicious foods.
impl ExecutedStep {
    fn machine_times(&self, world: &World) -> HashMap<Arc<str>, Number<Seconds>> {
        self.single_machine_time
            .iter()
            .map(|(machine, &single_machine_time)| {
                let time = single_machine_time / world.machines[machine].convert::<Seconds>();
                (machine.clone(), time)
            })
            .collect()
    }
    pub fn execute(&self, world: &mut World) {
        let times = self.machine_times(world);
        for (item, &amount) in &self.crafts {
            log::debug!("Crafted {amount:?} of {item:?}");
            *world.total_crafts.entry(item.clone()).or_default() += amount;
        }
        for (machine, &amount) in &self.builds {
            log::debug!("Built {amount:?} of {machine:?}");
            *world.machines.entry(machine.clone()).or_default() += amount;
        }
        log::debug!("Machine times: {times:#?}");
        for time in times.values().copied() {
            world.total_machine_time += time;
        }
        let total_time = times.into_values().max().unwrap_or_default();
        log::debug!("Step total time: {total_time:?}");
        world.time += total_time;
        log::debug!("Time now is {:?}", world.time);
    }

    fn log(&self, world: &World) {
        for (machine, amount) in &self.builds {
            log::info!("Built {amount:?} of {machine:?}");
        }
        let mut times = self.machine_times(world).into_iter().collect_vec();
        times.sort_by_key(|(_, time)| *time);
        for (machine, time) in times {
            log::info!(
                "{:?} {machine:?} worked for {time:?}",
                world.machines[&machine],
            );
        }
    }
}
