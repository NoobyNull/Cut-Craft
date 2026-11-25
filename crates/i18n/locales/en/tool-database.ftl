# Tool Database window: sentences with a value put in. Its other words are in text.ftl.

tool-database-no-cutting-data = No cutting data for { $material }: it cuts with its class's or its default data
tool-database-tool-changer = Tool changer: { $machine }
tool-database-back-to-default = Back to this type's default: { $default }
tool-database-chip-load = Chip load: { $load }
tool-database-loaded-in = Loaded in { $machine }
tool-database-no-data-yet = No cutting data for { $material } yet: the default's is shown, and a change keeps it as { $material }'s own
tool-database-points = { $count ->
    [one] { $count } point
   *[other] { $count } points
}
tool-database-from-slot = from slot T{ $slot } of { $machine }'s tool changer
tool-database-taken-as = not set: taken as { $shown }
tool-database-from-class-range = { $tool }: from its { $class } range, feed { $feed } mm/min at { $speed } r.p.m.
tool-database-class-span = From { $soft } ({ $soft_hardness }) to { $hard } ({ $hard_hardness }), { $scale }: a material between cuts with feeds between, by its hardness, unless it has feeds of its own
