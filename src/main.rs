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
            recipe_mode: RecipeMode::Expensive,
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
            let time = Number::new(1.0) / UPS; // TODO check if there is configuration
            self.use_energy(
                &boiler.energy_source.clone(),
                boiler.energy_consumption,
                time,
            );
            return;
        }

        panic!("Don't know how to produce {item:?}");
    }
    fn craft(&mut self, recipe: &Name, amount: Number) {
        log::debug!("Crafting {amount:?} of {recipe:?}");
        let data = self.data.clone();
        let recipe = &data
            .recipe
            .get(recipe)
            .unwrap_or_else(|| panic!("recipe not found {recipe:?}"))
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
    }
    fn mine(&mut self, resource: &Name, amount: Number) {
        let data = self.data.clone();
        let resource = data
            .resource
            .get(resource)
            .unwrap_or_else(|| panic!("resource not found {resource:?}"));
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
    }

    fn use_energy(&mut self, source: &EnergySource, usage: Number<Watts>, time: Number) {
        let joules = Number::<Joules>::new(
            usage.value() * time.value() / source.effectivity.as_ref().map_or(1.0, Number::value),
        );
        self.pollute(source.emissions_per_minute * time);
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
                self.ensure_have(
                    &item.name.clone(),
                    Number::new(fuel.value.value() / joules.value()),
                );
            }
            EnergyType::Electric => {
                let generator = self
                    .things_used
                    .iter()
                    .find_map(|name| self.data.generator.get(name))
                    .unwrap_or_else(|| panic!("no generator"));
                self.ensure_have(
                    &generator.fluid_box.filter.clone(),
                    generator.fluid_usage_per_tick * UPS,
                );
            }
            EnergyType::Heat => todo!(),
        }
    }

    fn pollute(&mut self, pollution: Number) {
        self.pollution += pollution;
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

    world.craft(&Name::from("automation-science-pack"), Number::new(10.0));

    Ok(())
}
