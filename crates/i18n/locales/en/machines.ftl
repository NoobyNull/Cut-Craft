# Machine Configuration and Post-Processor windows: sentences with a value put in. Their
# other words are in text.ftl.

machines-showing = Showing { $shown } of { $total }
machines-delete-post = Delete { $name }? Its file is removed, which cannot be undone.
machines-post-renamed = Renamed { $name }: machines using { $old } need it chosen again
machines-post-will-not-load = Will not load: { $why }
machines-centre-line = Centre line { $across }
machines-centre-line-hint = From work zero across to the turning axis, in { $across }. 0 when you zero { $across } on the axis, as is usual. (Its height comes from Job Setup: Z zero on the turning axis or on the material's top)
machines-bit-changes = { $grouped ->
    [yes] Bit Changes: { $how }, grouped by bit
   *[no] Bit Changes: { $how }
}
machines-both-arms = { $name } (both arms)
