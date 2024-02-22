use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    data::{EnergyType, FuelCategory, RecipeMode, Seconds, UPS},
    number::Number,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    BurnableFuelEnergy(FuelCategory),
    Mining(Arc<str>),
    Craft(Arc<str>),
    Research,
    Generator(Arc<str>),
    Boiler(Arc<str>),
    Free,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    Item {
        name: Arc<str>,
    },
    Energy {
        fuel_category: Option<crate::data::FuelCategory>,
        energy_type: crate::data::EnergyType,
    },
}

impl From<&str> for Item {
    fn from(value: &str) -> Self {
        Self::Item { name: value.into() }
    }
}

impl Item {
    pub fn name(&self) -> &Arc<str> {
        match self {
            Item::Item { name } => name,
            Item::Energy { .. } => panic!("energy item is fake item, no name for you, sorry"),
        }
    }
}

#[derive(Debug)]
pub struct Recipe {
    pub name: Arc<str>,
    pub category: Category,
    pub ingredients: HashMap<Item, Number>,
    pub results: HashMap<Item, Number>,
    /// `None` = instant
    pub crafting_time: Option<Number>,
}

#[derive(Debug)]
pub struct Machine {
    pub name: Arc<str>,
    pub categories: HashSet<Category>,
    pub energy_usage: HashMap<Item, Number>,
    pub crafting_speed: Number,
}

#[derive(Debug)]
pub struct Research {
    pub name: Arc<str>,
    pub dependencies: Vec<Arc<str>>,
    pub recipe: Arc<str>,
}

#[derive(Debug)]
pub struct Data {
    pub recipes: HashMap<Arc<str>, Recipe>,
    pub machines: HashMap<Arc<str>, Machine>,
    pub researches: HashMap<Arc<str>, Research>,
}

#[derive(Clone)]
pub struct World {
    data: Arc<Data>,
    researches: HashSet<Arc<str>>,
    preferred_fuel: HashMap<FuelCategory, Item>,
    machines: HashMap<Arc<str>, Number>,
    time: Number<Seconds>,
}

#[derive(Debug, Default)]
struct Step {
    crafts: HashMap<Arc<str>, Number>,
    builds: HashMap<Arc<str>, Number>,
    single_machine_time: HashMap<Arc<str>, Number<Seconds>>,
}

#[derive(Debug, Default)]
struct Plan {
    steps: Vec<Step>,
}

impl World {
    pub fn new(mode: RecipeMode, science_multiplier: impl Into<Number>) -> anyhow::Result<Self> {
        // running `factorio --dump-data``
        // will create `~/.factorio/script-output/data-raw-dump.json`
        let raw = crate::data::Data::from_reader(std::io::BufReader::new(std::fs::File::open(
            "data-raw-dump.json",
        )?))?;

        let mut data = Data {
            recipes: Default::default(),
            machines: Default::default(),
            researches: Default::default(),
        };

        let mut machines = HashMap::new();

        for recipe in raw.recipe.values() {
            let name = recipe.name.arc();
            let recipe = &recipe.modes[&mode];
            data.recipes.insert(
                name.clone(),
                Recipe {
                    name,
                    category: Category::Craft(recipe.category.arc()),
                    ingredients: recipe
                        .ingredients
                        .iter()
                        .map(|ingredient| {
                            (
                                Item::Item {
                                    name: ingredient.name.arc(),
                                },
                                ingredient.amount,
                            )
                        })
                        .collect(),
                    results: recipe
                        .results
                        .iter()
                        .map(|result| {
                            (
                                Item::Item {
                                    name: result.name.arc(),
                                },
                                result.amount * recipe.result_count.unwrap_or(1.into()),
                            )
                        })
                        .collect(),
                    crafting_time: Some(recipe.energy_required),
                },
            );
        }

        for item in raw.item.values() {
            if let Some(fuel) = &item.fuel {
                let name: Arc<str> =
                    format!("{:?} {:?} burnable fuel energy", item.name, fuel.category).into();
                data.recipes.insert(
                    name.clone(),
                    Recipe {
                        category: Category::BurnableFuelEnergy(fuel.category),
                        name,
                        ingredients: HashMap::from_iter([(
                            Item::Item {
                                name: item.name.arc(),
                            },
                            1.into(),
                        )]),
                        results: HashMap::from_iter([(
                            Item::Energy {
                                fuel_category: Some(fuel.category),
                                energy_type: crate::data::EnergyType::Burner,
                            },
                            fuel.value.value().into(),
                        )]),
                        crafting_time: None,
                    },
                );
            }
        }

        for resource in raw.resource.values() {
            let name: Arc<str> = format!("{:?} mining", resource.name).into();
            data.recipes.insert(
                name.clone(),
                Recipe {
                    name,
                    category: Category::Mining(resource.category.arc()),
                    ingredients: resource
                        .minable
                        .required_fluid
                        .iter()
                        .map(|fluid| {
                            (
                                Item::Item {
                                    name: fluid.name.arc(),
                                },
                                fluid.amount,
                            )
                        })
                        .collect(),
                    results: resource
                        .minable
                        .results
                        .iter()
                        .map(|result| {
                            (
                                Item::Item {
                                    name: result.name.arc(),
                                },
                                result.amount,
                            )
                        })
                        .collect(),
                    crafting_time: Some(resource.minable.mining_time),
                },
            );
        }

        fn energy_ingredients(
            source: &crate::data::EnergySource,
            usage: Number<crate::data::Watts>,
        ) -> HashMap<Item, Number> {
            let item = Item::Energy {
                fuel_category: source.fuel_category,
                energy_type: source.r#type,
            };
            let amount =
                Number::new(usage.value()) / source.effectivity.unwrap_or_else(|| 1.into());
            HashMap::from_iter([(item, amount)])
        }

        for drill in raw.mining_drill.values() {
            let name = drill.name.arc();
            data.machines.insert(
                name.clone(),
                Machine {
                    name,
                    categories: HashSet::from_iter(
                        drill
                            .resource_categories
                            .iter()
                            .map(|resource| Category::Mining(resource.arc())),
                    ),
                    crafting_speed: drill.mining_speed,
                    energy_usage: energy_ingredients(&drill.energy_source, drill.energy_usage),
                },
            );
        }

        for lab in raw.lab.values() {
            let name = lab.name.arc();
            data.machines.insert(
                name.clone(),
                Machine {
                    name,
                    categories: HashSet::from_iter([Category::Research]),
                    energy_usage: energy_ingredients(&lab.energy_source, lab.energy_usage),
                    crafting_speed: lab.researching_speed,
                },
            );
        }

        for generator in raw.generator.values() {
            let name = generator.name.arc();
            let recipe_name: Arc<str> = format!("generator {name:?} work").into();
            data.machines.insert(
                name.clone(),
                Machine {
                    name: name.clone(),
                    categories: HashSet::from_iter([Category::Generator(name.clone())]),
                    energy_usage: HashMap::new(),
                    crafting_speed: 1.into(),
                },
            );

            let fluid_name = generator.fluid_box.filter.clone();
            let fluid = &raw.fluid[&fluid_name];

            data.recipes.insert(
                recipe_name.clone(),
                Recipe {
                    name: recipe_name,
                    category: Category::Generator(name.clone()),
                    ingredients: HashMap::from_iter([(
                        Item::Item {
                            name: fluid.name.arc(),
                        },
                        generator.fluid_usage_per_tick,
                    )]),
                    results: HashMap::from_iter([(
                        Item::Energy {
                            fuel_category: None,
                            energy_type: EnergyType::Electric,
                        },
                        {
                            // https://wiki.factorio.com/Prototype/Generator#fluid_usage_per_tick
                            Number::new(
                                (std::cmp::min(
                                    generator.maximum_temperature,
                                    fluid.max_temperature.unwrap_or(Number::new(1e9)),
                                ) - fluid.default_temperature)
                                    .value(),
                            ) * generator.fluid_usage_per_tick
                                * Number::new(fluid.heat_capacity.unwrap().value())
                                * generator.effectivity
                        },
                    )]),
                    crafting_time: Some(Number::new(1.0) / UPS), // 1 tick
                },
            );
        }

        for boiler in raw.boiler.values() {
            let name = boiler.name.arc();
            data.machines.insert(
                name.clone(),
                Machine {
                    name: name.clone(),
                    categories: HashSet::from_iter([Category::Boiler(name.clone())]),
                    energy_usage: energy_ingredients(
                        &boiler.energy_source,
                        boiler.energy_consumption,
                    ),
                    crafting_speed: 1.into(),
                },
            );
            let recipe_name: Arc<str> = format!("boiling in {name:?}").into();
            data.recipes.insert(
                recipe_name.clone(),
                Recipe {
                    name: recipe_name,
                    category: Category::Boiler(name.clone()),
                    ingredients: HashMap::from_iter([(
                        Item::Item {
                            name: boiler.fluid_box.filter.arc(),
                        },
                        1.into(),
                    )]),
                    results: HashMap::from_iter([(
                        Item::Item {
                            name: boiler.output_fluid_box.filter.arc(),
                        },
                        1.into(),
                    )]),
                    crafting_time: Some(Number::new(1.0) / UPS), // TODO check if there is configuration,
                },
            );
        }

        for assembler in raw.assembling_machine.values() {
            let name = assembler.name.arc();
            data.machines.insert(
                name.clone(),
                Machine {
                    name,
                    categories: assembler
                        .crafting_categories
                        .iter()
                        .map(|name| Category::Craft(name.arc()))
                        .collect(),
                    crafting_speed: assembler.crafting_speed,
                    energy_usage: energy_ingredients(
                        &assembler.energy_source,
                        assembler.energy_usage,
                    ),
                },
            );
        }

        {
            let name: Arc<str> = "character mining".into();
            machines.insert(name.clone(), 1.into());
            data.machines.insert(
                name.clone(),
                Machine {
                    name,
                    categories: raw
                        .character
                        .mining_categories
                        .iter()
                        .map(|name| Category::Mining(name.arc()))
                        .collect(),
                    energy_usage: HashMap::new(),
                    crafting_speed: raw.character.mining_speed,
                },
            );
        }

        {
            let name: Arc<str> = "character crafting".into();
            machines.insert(name.clone(), 1.into());
            data.machines.insert(
                name.clone(),
                Machine {
                    name,
                    categories: raw
                        .character
                        .crafting_categories
                        .iter()
                        .map(|name| Category::Craft(name.arc()))
                        .collect(),
                    energy_usage: HashMap::new(),
                    crafting_speed: Number::from(1), // TODO: check, its not configurable?
                },
            );
        }

        for technology in raw.technology.values() {
            let name = technology.name.arc();
            let recipe_name: Arc<str> = format!("research {name:?}").into();
            data.researches.insert(
                name.clone(),
                Research {
                    name,
                    dependencies: technology
                        .prerequisites
                        .iter()
                        .map(|name| name.arc())
                        .collect(),
                    recipe: recipe_name.clone(),
                },
            );
            let count = match technology.unit.count {
                crate::data::TechnologyCount::Const { count } => count,
                crate::data::TechnologyCount::Formula { .. } => {
                    // TODO
                    continue;
                }
            };
            data.recipes.insert(
                recipe_name.clone(),
                Recipe {
                    name: recipe_name,
                    category: Category::Research,
                    ingredients: technology
                        .unit
                        .ingredients
                        .iter()
                        .map(|ingredient| {
                            (
                                Item::Item {
                                    name: ingredient.name.arc(),
                                },
                                ingredient.amount * count,
                            )
                        })
                        .collect(),
                    results: HashMap::new(),
                    crafting_time: Some(Number::new(technology.unit.time.value()) * count),
                },
            );
        }

        {
            let name: Arc<str> = "free".into();
            machines.insert(name.clone(), 1.into());
            data.machines.insert(
                name.clone(),
                Machine {
                    name,
                    categories: HashSet::from_iter([Category::Free]),
                    energy_usage: HashMap::new(),
                    crafting_speed: 1.into(),
                },
            );

            #[allow(clippy::single_element_loop)]
            for item in ["water"] {
                let recipe_name: Arc<str> = item.into();
                data.recipes.insert(
                    recipe_name.clone(),
                    Recipe {
                        name: recipe_name,
                        category: Category::Free,
                        ingredients: HashMap::new(),
                        results: HashMap::from_iter([(item.into(), 1.into())]),
                        crafting_time: None,
                    },
                );
            }
        }

        log::trace!("{data:#?}");

        Ok(Self {
            data: Arc::new(data),
            machines,
            preferred_fuel: HashMap::new(),
            researches: HashSet::new(),
            time: Number::new(0.0),
        })
    }

    pub fn craft(&mut self, item: impl Into<Item>, amount: impl Into<Number>) {
        self.planner().craft(item, amount).think().execute(self);
    }

    pub fn build(&mut self, machine: impl Into<Item>, amount: impl Into<Number>) {
        self.planner().build(machine, amount).think().execute(self);
    }

    pub fn prefer_fuel(&mut self, category: FuelCategory, item: impl Into<Item>) {
        self.preferred_fuel.insert(category, item.into());
    }

    pub fn destroy_all(&mut self, machine: impl Into<Item>) {
        let machine = machine.into();
        self.machines.remove(machine.name());
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
            .craft_recipe(format!("research {:?}", research.name), 1)
            .think()
            .execute(self);
        self.researches.insert(research.name.clone());
        log::info!("researched {:?}", research.name);
    }

    pub fn planner(&self) -> Planner<'_> {
        Planner {
            world: self,
            build: Default::default(),
            craft: Default::default(),
            craft_recipe: Default::default(),
        }
    }
}

struct Planner<'a> {
    world: &'a World,
    build: HashMap<Item, Number>,
    craft: HashMap<Item, Number>,
    craft_recipe: HashMap<Arc<str>, Number>,
}

impl Planner<'_> {
    pub fn build(&mut self, machine: impl Into<Item>, amount: impl Into<Number>) -> &mut Self {
        let amount = amount.into();
        let machine = machine.into();
        *self.build.entry(machine).or_default() += amount;
        self
    }
    pub fn craft(&mut self, item: impl Into<Item>, amount: impl Into<Number>) -> &mut Self {
        let amount = amount.into();
        let item = item.into();
        *self.craft.entry(item).or_default() += amount;
        self
    }
    pub fn craft_recipe(
        &mut self,
        recipe: impl Into<Arc<str>>,
        amount: impl Into<Number>,
    ) -> &mut Self {
        let amount = amount.into();
        let recipe = recipe.into();
        *self.craft_recipe.entry(recipe).or_default() += amount;
        self
    }
    pub fn think(&mut self) -> Plan {
        let mut step = StepPlanner {
            world: self.world,
            step: Step::default(),
        };
        for (machine, amount) in self.build.clone() {
            step.build(machine, amount);
        }
        for (item, amount) in self.craft.clone() {
            step.craft(item, amount);
        }
        for (recipe, amount) in self.craft_recipe.clone() {
            step.craft_recipe(recipe, amount);
        }
        Plan {
            steps: vec![step.finalize()],
        }
    }
}

struct StepPlanner<'a> {
    world: &'a World,
    step: Step,
}

impl StepPlanner<'_> {
    fn finalize(mut self) -> Step {
        let data = self.world.data.clone();
        let mut done = false;
        let mut total_times = HashMap::<Arc<str>, Number<Seconds>>::new();
        while !done {
            done = true;
            for (machine_name, single_machine_time) in
                std::mem::take(&mut self.step.single_machine_time)
            {
                *total_times.entry(machine_name.clone()).or_default() += single_machine_time;
                let machine = &data.machines[&machine_name];
                for (energy_item, &usage) in &machine.energy_usage {
                    let energy_amount = usage * single_machine_time.convert::<()>();
                    if energy_amount.value() < 1e-5 {
                        continue;
                    }
                    done = false;
                    self.craft(energy_item.clone(), energy_amount);
                }
            }
        }
        self.step.single_machine_time = total_times;
        self.step
    }
    fn build(&mut self, machine: Item, amount: Number) {
        *self.step.builds.entry(machine.name().clone()).or_default() += amount;
        self.craft(machine, amount);
    }
    fn find_recipe_for(&mut self, item: impl Into<Item>) -> Arc<str> {
        let item = item.into();

        if let Item::Energy {
            fuel_category: Some(category),
            energy_type: EnergyType::Burner,
        } = item
        {
            let fuel_item = self
                .world
                .preferred_fuel
                .get(&category)
                .unwrap_or_else(|| panic!("No preferred fuel set for {category:?}"));
            // TODO this format is copypasta
            return format!("{:?} {:?} burnable fuel energy", fuel_item.name(), category).into();
        }

        // TODO cache maybe?
        self.world
            .data
            .recipes
            .values()
            .filter(|recipe| recipe.results.contains_key(&item))
            .filter(|recipe| {
                self.world.machines.keys().any(|machine| {
                    self.world.data.machines[machine]
                        .categories
                        .contains(&recipe.category)
                })
            })
            .filter(|recipe| !recipe.name.contains("barrel"))
            .filter(|recipe| &*recipe.name != "coal-liquefaction")
            .max_by_key(|recipe| {
                (
                    matches!(recipe.category, Category::Free),
                    &*recipe.name == "advanced-oil-processing",
                )
            })
            .map(|recipe| recipe.name.clone())
            .unwrap_or_else(|| panic!("Could not find recipe for {item:?}"))
    }
    fn craft(&mut self, item: Item, amount: Number) {
        let recipe = self.find_recipe_for(item.clone());
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
        *self.step.crafts.entry(recipe.name.clone()).or_default() += crafts;

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
                    .step
                    .single_machine_time
                    .entry(machine_name.clone())
                    .or_default() += single_machine_time.convert::<Seconds>();
            }
        }
    }
}

impl Plan {
    pub fn execute(self, world: &mut World) {
        for step in self.steps {
            step.execute(world);
        }
    }
}

impl Step {
    pub fn execute(self, world: &mut World) {
        let times: HashMap<Arc<str>, Number<Seconds>> = self
            .single_machine_time
            .into_iter()
            .map(|(machine, single_machine_time)| {
                let time = single_machine_time / world.machines[&machine].convert::<Seconds>();
                (machine, time)
            })
            .collect();
        for (item, amount) in self.crafts {
            log::debug!("Crafted {amount:?} of {item:?}");
        }
        for (machine, amount) in self.builds {
            log::debug!("Built {amount:?} of {machine:?}");
            *world.machines.entry(machine).or_default() += amount;
        }
        log::debug!("Machine times: {times:#?}");
        let total_time = times.into_values().max().unwrap();
        log::debug!("Step total time: {total_time:?}");
        world.time += total_time;
        log::info!("Time now is {:?}", world.time);
    }
}
