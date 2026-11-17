# Permission nodes

A permission node is a named, typed question a server mod asks about a player's UUID, for example whether a
player may harvest a rare crop. Nodes are server-side only. Each node has a default answer, and that default
answers every query unless the server selects another handler. Nodes are separate from operator levels: a node
grant does not give a player an operator level, a node denial does not remove one, and vanilla's own commands
and checks do not consult nodes.

## Declaring nodes

Add each node in the `permission_gather_nodes` hook through its collector, `PermissionNodes::add(node, default)`.
Gathering runs once per logical server start, after operator levels load and before the server starts. There is
no late registration: a node added after that is refused.

- Identity is the node's case-sensitive dot-separated name plus its value type. Boolean, integer, string and
  text-component nodes are distinct nodes even under one name.
- Adding the same typed node twice is a registration error, and no partial service is published.
- A Boolean node's default is the operator level that grants it by default. Other types take a default value.
- The default is a resolver the mod writes. It runs at query time, not at gathering time, and receives the
  online player when there is one, the player's UUID and the contexts the caller supplied. A declared context
  key the caller leaves out is allowed; an undeclared key is ignored.

## Handlers

Exactly one handler answers every node on a server. The built-in default handler answers from each node's
default. A mod can add its own handler with `permission_gather_handler`, which adds a factory under an id
(`factories.add(id, ...)`). Duplicate handler ids are registration errors.

The dedicated server's `permission-handler` setting names the handler to use. With no setting, or an unknown id,
the default handler is used, and the log names the unknown id; startup does not fail.

A handler can answer independently of operator levels. Its failures are never a grant: an error, a trap or a
recursive query is an explicit `PermissionError`, and the protected action must treat it as a refusal. Integer,
string and component errors must not be read as a Boolean denial.

## Querying

| Call | Behavior |
|---|---|
| `permission_query(node, uuid, contexts)` | Returns `Result<PermissionValue, PermissionError>`: the node's typed value from the selected handler. |
| `permission_bool(player, &node)` | The same for a Boolean node and a connected player, as `Result<bool, PermissionError>`. |

`PermissionError` names each refusal: `NotInitialized` (a query before the service is ready), `Unregistered`
(an unknown node, never an implicit registration), `DuplicateNode`, `DuplicateHandler`, `WrongType`,
`UnknownPlayer`, `HandlerFailed`, `Recursive` (a handler that queries inside its own query fails both queries)
and `InvalidInput`. A failed query changes nothing and grants nothing.

Queries take no cache and write nothing. A node decision does not fire `permissions_changed`, and that hook
does not report node decisions. There are no wildcards, no hierarchy and no namespace ownership: a node name
means exactly what it spells.

Put the check at the server-side point where the action happens, and refuse the action on any error.

## Commands that use a node

A mod command path may declare a Boolean node with `Command::with_node(&node)`. Its answer is decided in this
order:

1. The configured handler's answer decides.
2. With no handler configured, the usage's operator level decides, as in vanilla.
3. A handler error denies the path and is logged. It never falls back to the operator level.

A path whose check fails is left out of the command tree sent to that player, and typing it answers as an
unknown command. Commands without a node keep their operator level, and vanilla's commands are unchanged.

## Refreshing a player's command tree

`host.player_resend_command_tree(player)` sends one player their command tree again, as vanilla does on join,
respawn, dimension change and an operator change, with the mods' say on it. Call it when a handler's answers for
that player change. Only that player's client is sent a packet. The server checks every command run anyway, so a
stale tree affects only the suggestions the client shows.

It returns `Result<(), PlayerWorldRefusal>`, refusing with `Client` on a client, `Stale` for a player that is not
in the server's list (or a handle of an earlier open of the world) and `NoConnection` for a fake player. A
refusal sends nothing.

## What a node does not do

- A node does not change `/op`, the operator list, or any vanilla command or command-block check.
- A node is not a cache: each query asks the handler again.
- A handler cannot be chained with another handler; one answers each node.
