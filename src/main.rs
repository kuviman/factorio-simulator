mod data;
mod number;
use number::Number;

use std::{collections::HashMap, sync::Arc};

use data::*;

struct World {
    recipe_mode: RecipeMode,
    data: Arc<Data>,
    things_used: Vec<Name>,
    pollution: Number,
    total_evolution: f64,
    biters: HashMap<Name, Number>,
    items: HashMap<Name, Number>,
    total_produced: HashMap<Name, Number>,
}

impl World {
    pub fn new() -> anyhow::Result<Self> {
        // running `factorio --dump-data``
        // will create `~/.factorio/script-output/data-raw-dump.json`
        let data = Data::from_reader(std::io::BufReader::new(std::fs::File::open(
            "data-raw-dump.json",
        )?))?;
        Ok(Self {
            data: Arc::new(data),
            things_used: vec![
                "coal".into(),
                "burner-mining-drill".into(),
                "stone-furnace".into(),
                "assembling-machine-1".into(),
                "steam-engine".into(),
                "boiler".into(),
            ],
            recipe_mode: RecipeMode::Normal,
            total_evolution: 0.0,
            pollution: Number::new(0.0),
            biters: Default::default(),
            items: Default::default(),
            total_produced: Default::default(),
        })
    }
    fn add_item(&mut self, item: &Name, amount: Number) {
        *self.items.entry(item.clone()).or_default() += amount;
        if amount > Number::new(0.0) {
            *self.total_produced.entry(item.clone()).or_default() += amount;
        }
    }
    fn ensure_have(&mut self, item: &Name, amount: Number) {
        let current = self.items.get(item).copied().unwrap_or_default();
        if current < amount {
            self.produce(item, amount - current);
        }
    }
    fn produce(&mut self, item: &Name, amount: Number) {
        log::trace!("Trying to produce {amount:?} of {item:?}");
        let data = self.data.clone();

        if let Some(resource) = data.resource.get(item) {
            if let Some(result) = resource
                .minable
                .results
                .iter()
                .find(|result| result.name == *item)
            {
                self.mine(&resource.name.clone(), amount / result.amount);
            } else {
                panic!("Resource {:?} does not produce {item:?}", resource.name);
            }
            return;
        }

        if let Some(recipe) = data.recipe.get(item) {
            let recipe = &recipe.modes[&self.recipe_mode];
            if let Some(result) = recipe.results.iter().find(|result| result.name == *item) {
                self.craft(item, amount / result.amount);
            } else {
                panic!("{item:?} does not produce {item:?}");
            }
            return;
        }

        if let Some(boiler) = self.things_used.iter().find_map(|name| {
            let boiler = data.boiler.get(name)?;
            (boiler.output_fluid_box.filter == *item).then_some(boiler)
        }) {
            let time = amount * Number::new(1.0) / UPS; // TODO check if there is configuration
            log::debug!(
                "Need to boil {amount:?} of {item:?} in {time:?} using {:?}",
                boiler.energy_consumption
            );
            self.use_energy(
                &boiler.energy_source.clone(),
                boiler.energy_consumption,
                time,
            );
            log::debug!("Boiled {amount:?} of {item:?} in {time:?}");
            return;
        }

        panic!("Don't know how to produce {item:?}");
    }
    fn craft(&mut self, recipe_name: &Name, amount: Number) {
        let data = self.data.clone();
        let recipe = &data
            .recipe
            .get(recipe_name)
            .unwrap_or_else(|| panic!("recipe not found {recipe_name:?}"))
            .modes[&self.recipe_mode];
        log::trace!("Recipe: {recipe:#?}");
        for ingredient in &recipe.ingredients {
            let ingredient_amount = ingredient.amount * amount;
            self.ensure_have(&ingredient.name, ingredient_amount);
            self.add_item(&ingredient.name, ingredient_amount);
        }
        for result in &recipe.results {
            let result_amount = result.amount * amount;
            self.add_item(&result.name, result_amount);
        }
        let category = &recipe.category;
        let assembler = self
            .things_used
            .iter()
            .find_map(|name| {
                let assembler = data.assembling_machine.get(name)?;
                assembler
                    .crafting_categories
                    .contains(category)
                    .then_some(assembler)
            })
            .unwrap_or_else(|| panic!("no assembler for {category:?}"));
        let time = amount * recipe.energy_required / assembler.crafting_speed;
        self.use_energy(
            &assembler.energy_source.clone(),
            assembler.energy_usage,
            time,
        );
        log::debug!("Crafted {amount:?} of {recipe_name:?} in {time:?}");
    }
    fn mine(&mut self, resource_name: &Name, amount: Number) {
        let data = self.data.clone();
        let resource = data
            .resource
            .get(resource_name)
            .unwrap_or_else(|| panic!("resource not found {resource_name:?}"));
        for result in &resource.minable.results {
            let result_amount = result.amount * amount;
            self.add_item(&result.name, result_amount);
        }
        let category = &resource.category;
        let miner = self
            .things_used
            .iter()
            .find_map(|name| {
                let drill = data.mining_drill.get(name)?;
                drill
                    .resource_categories
                    .contains(category)
                    .then_some(drill)
            })
            .unwrap_or_else(|| panic!("no drill for {category:?}"));
        let time = amount * resource.minable.mining_time / miner.mining_speed;
        self.use_energy(&miner.energy_source.clone(), miner.energy_usage, time);
        log::debug!("Mined {amount:?} of {resource_name:?} in {time:?}");
    }

    fn use_energy(&mut self, source: &EnergySource, usage: Number<Watts>, time: Number) {
        let joules = Number::<Joules>::new(
            usage.value() * time.value() / source.effectivity.as_ref().map_or(1.0, Number::value),
        );
        match source.r#type {
            EnergyType::Burner => {
                let fuel_category = source.fuel_category.unwrap();
                let item = self
                    .things_used
                    .iter()
                    .find_map(|name| {
                        let item = self.data.item.get(name)?;
                        (fuel_category == item.fuel.as_ref()?.category).then_some(item)
                    })
                    .unwrap_or_else(|| panic!("no fuel for {fuel_category:?}"));
                let fuel = item.fuel.as_ref().unwrap();
                assert_eq!(fuel.category, fuel_category);
                let amount = Number::new(joules.value() / fuel.value.value());
                let item_name = item.name.clone();
                self.ensure_have(&item_name, amount);
                log::debug!("Burned {amount:?} of {item_name:?} to produce {joules:?}");
            }
            EnergyType::Electric => {
                let generator = self
                    .things_used
                    .iter()
                    .find_map(|name| self.data.generator.get(name))
                    .unwrap_or_else(|| panic!("no generator"));
                let fluid_name = generator.fluid_box.filter.clone();
                let fluid = &self.data.fluid[&fluid_name];

                // https://wiki.factorio.com/Prototype/Generator#fluid_usage_per_tick
                let max_power_output = Number::new(
                    (std::cmp::min(
                        generator.maximum_temperature,
                        fluid.max_temperature.unwrap_or(Number::new(1e9)),
                    ) - fluid.default_temperature)
                        .value(),
                ) * generator.fluid_usage_per_tick
                    * UPS // wiki says per tick
                    * Number::new(fluid.heat_capacity.unwrap().value())
                    * generator.effectivity;

                let generator_time = Number::new(joules.value()) / max_power_output;
                let amount = generator_time * generator.fluid_usage_per_tick * UPS;
                self.ensure_have(&fluid_name, amount);
                log::debug!(
                    "Used {amount:?} of {fluid_name:?} in {generator_time:?}, produced {joules:?}"
                );
            }
            EnergyType::Heat => todo!(),
        }
        self.pollute(source.emissions_per_minute / Number::new(60.0) * time);
    }

    fn pollute(&mut self, pollution: Number) {
        log::debug!("Made {pollution:?} pollution");
        self.pollution += pollution;
        self.total_evolution += pollution.value() * 9e-07; // TODO map_settings
    }

    fn evolution(&self) -> f64 {
        self.total_evolution / (1.0 + self.total_evolution)
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::new()
        .filter_level(if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .parse_default_env()
        .init();

    let mut world = World::new()?;

    for science in [
        10,     // auto
        10_000, // turret
        10_000, // mil 1
        75_000, // green science
        50_000, // steel
        10_000, // wall
        30_000, // electro
    ] {
        world.craft(
            &Name::from("automation-science-pack"),
            Number::new(science as f64),
        );
    }

    for science in [
        20,  // mil 2
        30,  // mil science
        40,  // auto 2
        50,  // fluid
        100, // oil
        50,  // flamable
        50,  // flamethrower
    ] {
        let science = science as f64 * 1000.0;
        world.craft(&Name::from("automation-science-pack"), Number::new(science));
        world.craft(&Name::from("logistic-science-pack"), Number::new(science));
    }
    world.craft(&Name::from("military-science-pack"), Number::new(50_000.0));
    log::info!("evolution = {:.1}%", world.evolution() * 100.0);

    Ok(())
}
