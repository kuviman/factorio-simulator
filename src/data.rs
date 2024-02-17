use crate::number::{Number, NumberType};
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
};

use anyhow::anyhow;
use serde::Deserialize;

pub const UPS: Number = Number::new(60.0);

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum FuelCategory {
    Chemical,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum EnergyType {
    Burner,
    Electric,
    Heat,
}

#[derive(Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(from = "String")]
pub struct Name(Arc<str>);

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl NumberType for () {
    const SUFFIX: Option<&'static str> = None;
}

pub struct Seconds;

impl NumberType for Seconds {
    const PARSE_SUFFIX: bool = false;
    const SUFFIX: Option<&'static str> = Some("s");
}

pub struct Joules;

impl NumberType for Joules {
    const SUFFIX: Option<&'static str> = Some("J");
}

pub struct Watts;

impl NumberType for Watts {
    const SUFFIX: Option<&'static str> = Some("W");
}

pub struct Temperature;

impl NumberType for Temperature {
    const PARSE_SUFFIX: bool = false;
    const SUFFIX: Option<&'static str> = Some("°");
}

#[derive(Debug, Deserialize)]
pub struct Fuel {
    #[serde(rename = "fuel_value")]
    pub value: Number<Joules>,
    #[serde(rename = "fuel_category")]
    pub category: FuelCategory,
}

#[derive(Debug, Deserialize)]
pub struct Color {
    #[serde(default)]
    pub r: f64,
    #[serde(default)]
    pub g: f64,
    #[serde(default)]
    pub b: f64,
}

#[derive(Debug, Deserialize)]
pub struct Item {
    pub name: Name,
    pub stack_size: usize,
    #[serde(flatten)]
    pub fuel: Option<Fuel>,
}

#[derive(Debug, Deserialize)]
pub struct Fluid {
    pub name: Name,
    pub default_temperature: Number<Temperature>,
    pub max_temperature: Option<Number<Temperature>>,
    pub heat_capacity: Option<Number<Joules>>,
    pub base_color: Color,
    pub flow_color: Color,
}

#[derive(Debug, Deserialize)]
pub struct MinableResourceRequiredFluid {
    #[serde(rename = "required_fluid")]
    pub name: Name,
    #[serde(rename = "fluid_amount")]
    pub amount: Number,
}

#[derive(Debug, Deserialize)]
pub struct Minable {
    #[serde(flatten)]
    pub required_fluid: Option<MinableResourceRequiredFluid>,
    pub mining_time: Number,
    #[serde(alias = "result", deserialize_with = "deserialize_item_list")]
    pub results: Vec<AmountOf>,
}

#[derive(Debug, Deserialize)]
pub struct Resource {
    pub name: Name,
    #[serde(default = "basic_solid")] // TODO???
    pub category: Name,
    pub minable: Minable,
}

fn basic_solid() -> Name {
    "basic-solid".into()
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnergySource {
    pub r#type: EnergyType,
    #[serde(default)]
    pub emissions_per_minute: Number,
    pub effectivity: Option<Number>,
    pub fuel_category: Option<FuelCategory>,
}

#[derive(Debug, Deserialize)]
pub struct MiningDrill {
    pub name: Name,
    pub minable: Minable,
    pub resource_categories: HashSet<Name>,
    pub mining_speed: Number,
    pub energy_usage: Number<Watts>,
    pub energy_source: EnergySource,
}

#[derive(Debug, Deserialize)]
pub struct FluidBox {
    pub minimum_temperature: Option<Number>,
    pub filter: Name,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TechnologyCount {
    Const { count: Number },
    Formula { count_formula: String },
}

#[derive(Debug, Deserialize)]
pub struct TechnologyUnit {
    #[serde(flatten)]
    pub count: TechnologyCount,
    pub time: Number<Seconds>,
    #[serde(deserialize_with = "deserialize_item_list")]
    pub ingredients: Vec<AmountOf>,
}

#[derive(Debug, Deserialize)]
pub struct Technology {
    pub name: Name,
    pub unit: TechnologyUnit,
    #[serde(default)]
    pub prerequisites: Vec<Name>,
    #[serde(default)]
    pub ignore_tech_cost_multiplier: bool,
}

#[derive(Debug, Deserialize)]
pub struct Boiler {
    pub name: Name,
    pub minable: Minable,
    pub target_temperature: Number,
    pub fluid_box: FluidBox,
    pub output_fluid_box: FluidBox,
    pub energy_consumption: Number<Watts>,
    pub energy_source: EnergySource,
}

#[derive(Debug, Deserialize)]
pub struct Generator {
    pub name: Name,
    pub minable: Minable,
    pub effectivity: Number,
    pub fluid_usage_per_tick: Number,
    pub maximum_temperature: Number<Temperature>,
    pub min_perceived_performance: Number,
    pub fluid_box: FluidBox,
}

#[derive(Debug, Deserialize)]
pub struct AssemblingMachine {
    pub name: Name,
    pub minable: Minable,
    pub crafting_categories: HashSet<CraftCategory>,
    pub crafting_speed: Number,
    pub energy_usage: Number<Watts>,
    pub energy_source: EnergySource,
}

#[derive(Debug)]
pub struct AmountOf {
    pub name: Name,
    pub amount: Number,
}

#[derive(Default, Debug, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum CraftCategory {
    Smelting,
    OilProcessing,
    Chemistry,
    BasicCrafting,
    #[default] // TODO confirm?
    Crafting,
    AdvancedCrafting,
    CraftingWithFluid,
    RocketBuilding,
    Centrifuging,
}

#[derive(Debug, Deserialize)]
pub struct Recipe {
    #[serde(default)]
    pub category: CraftCategory,
    #[serde(deserialize_with = "deserialize_item_list")]
    pub ingredients: Vec<AmountOf>,
    #[serde(alias = "result", deserialize_with = "deserialize_item_list")]
    pub results: Vec<AmountOf>,
    #[serde(default = "default_recipe_energy")]
    pub energy_required: Number,
}

fn default_recipe_energy() -> Number {
    Number::new(0.5) // TODO confirm?
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum RecipeMode {
    Normal,
    Expensive,
}

#[derive(Debug, Deserialize)]
#[serde(from = "RecipeModesProxy")]
pub struct RecipeModes {
    pub name: Name,
    #[serde(deserialize_with = "deserialize_recipe_modes")]
    pub modes: HashMap<RecipeMode, Arc<Recipe>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RecipeModesProxy {
    AllSame {
        name: Name,
        #[serde(flatten)]
        recipe: Arc<Recipe>,
    },
    Split {
        name: Name,
        normal: Arc<Recipe>,
        expensive: Arc<Recipe>,
    },
}

impl From<RecipeModesProxy> for RecipeModes {
    fn from(modes: RecipeModesProxy) -> Self {
        let (name, normal, expensive) = match modes {
            RecipeModesProxy::AllSame { name, recipe } => (name, recipe.clone(), recipe),
            RecipeModesProxy::Split {
                name,
                normal,
                expensive,
            } => (name, normal, expensive),
        };
        let mut modes = HashMap::new();
        modes.insert(RecipeMode::Normal, normal);
        modes.insert(RecipeMode::Expensive, expensive);
        Self { name, modes }
    }
}

fn deserialize_item_list<'de, D>(deserializer: D) -> Result<Vec<AmountOf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Item {
        AmountOf(Name, Number),
        TypedAmountOf {
            r#type: Option<EntityType>,
            probability: Option<f64>,
            name: Name,
            #[serde(alias = "amount_min")] // TODO
            amount: Number,
            fluidbox_index: Option<usize>,
        },
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MaybeManyItems {
        Single(Name),
        Multiple(Vec<Item>),
    }
    let items = MaybeManyItems::deserialize(deserializer)?;
    let items = match items {
        MaybeManyItems::Single(name) => vec![Item::AmountOf(name, Number::new(1.0))],
        MaybeManyItems::Multiple(items) => items,
    };
    Ok(items
        .into_iter()
        .map(|item| match item {
            Item::AmountOf(name, amount) | Item::TypedAmountOf { name, amount, .. } => {
                AmountOf { name, amount }
            }
        })
        .collect())
}

#[derive(Debug, Deserialize)]
pub struct Tile {
    pub name: Name,
    pub map_color: Color,
    pub pollution_absorption_per_second: Number,
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum EntityType {
    Accumulator,
    Achievement,
    ActiveDefenseEquipment,
    AmbientSound,
    Ammo,
    AmmoCategory,
    AmmoTurret,
    ArithmeticCombinator,
    Armor,
    Arrow,
    ArtilleryFlare,
    ArtilleryProjectile,
    ArtilleryTurret,
    ArtilleryWagon,
    AssemblingMachine,
    AutoplaceControl,
    BatteryEquipment,
    Beacon,
    Beam,
    BeltImmunityEquipment,
    Blueprint,
    BlueprintBook,
    Boiler,
    BuildEntityAchievement,
    BurnerGenerator,
    Capsule,
    Car,
    CargoWagon,
    Character,
    CharacterCorpse,
    Cliff,
    CombatRobot,
    CombatRobotCount,
    ConstantCombinator,
    ConstructionRobot,
    ConstructWithRobotsAchievement,
    Container,
    CopyPasteTool,
    Corpse,
    CurvedRail,
    CustomInput,
    DamageType,
    DeciderCombinator,
    DeconstructibleTileProxy,
    DeconstructionItem,
    DeconstructWithRobotsAchievement,
    DeliverByRobotsAchievement,
    DontBuildEntityAchievement,
    DontCraftManuallyAchievement,
    DontUseEntityInEnergyProductionAchievement,
    EditorController,
    ElectricEnergyInterface,
    ElectricPole,
    ElectricTurret,
    EnergyShieldEquipment,
    EntityGhost,
    EquipmentCategory,
    EquipmentGrid,
    Explosion,
    FinishTheGameAchievement,
    Fire,
    Fish,
    FlameThrowerExplosion,
    Fluid,
    FluidTurret,
    FluidWagon,
    FlyingText,
    Font,
    FuelCategory,
    Furnace,
    Gate,
    Generator,
    GeneratorEquipment,
    GodController,
    GroupAttackAchievement,
    GuiStyle,
    Gun,
    HeatInterface,
    HeatPipe,
    HighlightBox,
    InfinityContainer,
    InfinityPipe,
    Inserter,
    Item,
    ItemEntity,
    ItemGroup,
    ItemRequestProxy,
    ItemSubgroup,
    ItemWithEntityData,
    ItemWithInventory,
    ItemWithLabel,
    ItemWithTags,
    KillAchievement,
    Lab,
    Lamp,
    LandMine,
    LeafParticle,
    LinkedBelt,
    LinkedContainer,
    Loader,
    #[serde(rename = "loader-1x1")]
    Loader1x1,
    Locomotive,
    LogisticContainer,
    LogisticRobot,
    MapGenPresets,
    MapSettings,
    Market,
    MiningDrill,
    MiningTool,
    Module,
    ModuleCategory,
    MouseCursor,
    MovementBonusEquipment,
    NightVisionEquipment,
    NoiseExpression,
    NoiseLayer,
    OffshorePump,
    OptimizedDecorative,
    OptimizedParticle,
    Particle,
    ParticleSource,
    Pipe,
    PipeToGround,
    PlayerDamagedAchievement,
    PlayerPort,
    PowerSwitch,
    ProduceAchievement,
    ProducePerHourAchievement,
    ProgrammableSpeaker,
    Projectile,
    Pump,
    Radar,
    RailChainSignal,
    RailPlanner,
    RailRemnants,
    RailSignal,
    Reactor,
    Recipe,
    RecipeCategory,
    RepairTool,
    ResearchAchievement,
    Resource,
    ResourceCategory,
    Roboport,
    RoboportEquipment,
    RocketSilo,
    RocketSiloRocket,
    RocketSiloRocketShadow,
    SelectionTool,
    Shortcut,
    SimpleEntity,
    SimpleEntityWithForce,
    SimpleEntityWithOwner,
    Smoke,
    SmokeWithTrigger,
    SolarPanel,
    SolarPanelEquipment,
    SpectatorController,
    SpeechBubble,
    SpiderLeg,
    SpidertronRemote,
    SpiderVehicle,
    Splitter,
    Sprite,
    Sticker,
    StorageTank,
    StraightRail,
    Stream,
    Technology,
    Tile,
    TileEffect,
    TileGhost,
    TipsAndTricksItem,
    TipsAndTricksItemCategory,
    Tool,
    TrainPathAchievement,
    TrainStop,
    TransportBelt,
    Tree,
    TriggerTargetType,
    TrivialSmoke,
    Turret,
    Tutorial,
    UndergroundBelt,
    Unit,
    UnitSpawner,
    UpgradeItem,
    UtilityConstants,
    UtilitySounds,
    UtilitySprites,
    VirtualSignal,
    Wall,
    WindSound,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Prototype {
    Item(Item),
    Tile(Tile),
    Fluid(Fluid),
    Recipe(RecipeModes),
    Resource(Resource),
    MiningDrill(MiningDrill),
    AssemblingMachine(AssemblingMachine),
    Furnace(AssemblingMachine),
    Generator(Generator),
    Boiler(Boiler),
    Technology(Technology),
    #[serde(other)]
    Other,
}

#[derive(Default, Debug, Deserialize)]
pub struct Data {
    pub item: HashMap<Name, Item>,
    pub tile: HashMap<Name, Tile>,
    pub fluid: HashMap<Name, Fluid>,
    pub recipe: HashMap<Name, RecipeModes>,
    pub resource: HashMap<Name, Resource>,
    pub mining_drill: HashMap<Name, MiningDrill>,
    pub assembling_machine: HashMap<Name, AssemblingMachine>,
    pub generator: HashMap<Name, Generator>,
    pub boiler: HashMap<Name, Boiler>,
    pub technology: HashMap<Name, Technology>,
    pub other: HashMap<EntityType, HashSet<Name>>,
}

impl Data {
    pub fn from_reader(reader: impl std::io::Read) -> anyhow::Result<Self> {
        let raw: HashMap<EntityType, HashMap<Name, Prototype>> = serde_json::from_reader(reader)?;
        let mut data = Self::default();
        for (entity_type, prototypes) in raw {
            for (name, prototype) in prototypes {
                match prototype {
                    Prototype::Item(item) => {
                        data.item.insert(name, item);
                    }
                    Prototype::Tile(tile) => {
                        data.tile.insert(name, tile);
                    }
                    Prototype::Fluid(fluid) => {
                        data.fluid.insert(name, fluid);
                    }
                    Prototype::Recipe(recipe) => {
                        data.recipe.insert(name, recipe);
                    }
                    Prototype::Resource(resource) => {
                        data.resource.insert(name, resource);
                    }
                    Prototype::MiningDrill(mining_drill) => {
                        data.mining_drill.insert(name, mining_drill);
                    }
                    Prototype::AssemblingMachine(assembling_machine) => {
                        data.assembling_machine.insert(name, assembling_machine);
                    }
                    Prototype::Furnace(furnace) => {
                        data.assembling_machine.insert(name, furnace);
                    }
                    Prototype::Generator(generator) => {
                        data.generator.insert(name, generator);
                    }
                    Prototype::Boiler(boiler) => {
                        data.boiler.insert(name, boiler);
                    }
                    Prototype::Technology(technology) => {
                        data.technology.insert(name, technology);
                    }
                    Prototype::Other => {
                        data.other.entry(entity_type).or_default().insert(name);
                    }
                }
            }
        }
        Ok(data)
    }
}
