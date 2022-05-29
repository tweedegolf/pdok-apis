# Geo APIs for Pect

## Usage

Exposed functions expect a client object for calling upstreams.

For `locatieserver`:

```
let lookup_client = lookup::LookupClient::new("Your user agent");
let suggestions = lookup_client.suggest_concrete("6512EX", "26").await?;
...
```

For `bag`:

```
let bag_client = bag::BagClient::new("Your user agent", "Your BAG key");
let buildings = bag.get_panden("0268010000084126").await?;
...
```

For `brk`:

```
let brk_client = brk::BrkClient::new("Your user agent", brk::CoordinateSpace::Gps);
let lot = brk_client.get_lot("HTT02", "M", "5038").await?;
...
```

## Test upstream

Test if upstreams produce expected output:

```
BAG_API_KEY=<your key> cargo test
```