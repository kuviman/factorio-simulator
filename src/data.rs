use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    number::Number,
    raw_data::{EnergyType, FuelCategory, RecipeMode, UPS},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    BurnableFuelEnergy(FuelCategory),
    PickaxeMining,
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
        fuel_category: Option<crate::raw_data::FuelCategory>,
        energy_type: crate::raw_data::EnergyType,
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
    pub fn is(&self, test_name: &str) -> bool {
        match self {
            Item::Item { name } => &**name == test_name,
            Item::Energy { .. } => false,
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

pub const CHARACTER_MINING: &str = "character mining";
pub const CHARACTER_CRAFTING: &str = "character crafting";
pub const FREE_STUFF: &str = "free";

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

impl Data {
    pub fn new(mode: RecipeMode) -> anyhow::Result<Self> {
        // running `factorio --dump-data`
        // will create `~/.factorio/script-output/data-raw-dump.json`
        let raw = crate::raw_data::Data::from_reader(std::io::BufReader::new(
            std::fs::File::open("data-raw-dump.json")?,
        ))?;

        let mut data = Data {
            recipes: Default::default(),
            machines: Default::default(),
            researches: Default::default(),
        };

        for simple_entity in raw.simple_entity.values() {
            if simple_entity.count_as_rock_for_filtered_deconstruction {
                let name: Arc<str> = format!("pickaxe mine {:?}", simple_entity.name).into();
                // its a rock, its minable, yea
                let minable = simple_entity.minable.as_ref().unwrap();
                data.recipes.insert(
                    name.clone(),
                    Recipe {
                        name,
                        category: Category::PickaxeMining,
                        ingredients: HashMap::new(),
                        results: minable
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
                        crafting_time: Some(minable.mining_time),
                    },
                );
            }
        }

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
                                energy_type: crate::raw_data::EnergyType::Burner,
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
            source: &crate::raw_data::EnergySource,
            usage: Number<crate::raw_data::Watts>,
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

        // TODO: maybe merge character mining & crafting into 1 machine?
        {
            let name: Arc<str> = CHARACTER_MINING.into();
            data.machines.insert(
                name.clone(),
                Machine {
                    name,
                    categories: raw
                        .character
                        .mining_categories
                        .iter()
                        .map(|name| Category::Mining(name.arc()))
                        .chain([Category::PickaxeMining])
                        .collect(),
                    energy_usage: HashMap::new(),
                    crafting_speed: raw.character.mining_speed,
                },
            );
        }

        {
            let name: Arc<str> = CHARACTER_CRAFTING.into();
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
                crate::raw_data::TechnologyCount::Const { count } => count,
                crate::raw_data::TechnologyCount::Formula { .. } => {
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
            let name: Arc<str> = FREE_STUFF.into();
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
        Ok(data)
    }
}
