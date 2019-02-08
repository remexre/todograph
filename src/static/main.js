fetch("./api/all")
  .then(r => r.json())
  .then(g => { console.log(g); return g; })
  .then(g => ({
    edges: g.edges.map((e, i) => ({ id: i.toString(), source: e.from.toString(), target: e.to.toString() })),
    nodes: g.todos.map(t => ({ id: t.id.toString(), label: t.name }))
  }))
  .then(g => new sigma({ container: 'main', graph: g }))
