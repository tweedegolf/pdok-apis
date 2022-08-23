# PDOK APIs

Provides partial support for consuming APIs of PDOK. These APIs provide information on addresses, buildings and lots in the Netherlands.

## Usage

For finding an address information using a postal code and housenumber, `locatieserver`:

```
let lookup_client = lookup::LookupClient::new("Your user agent");
let suggestions = lookup_client.suggest_concrete("6512EX", "26").await?;
...
```

For getting building information, `bag`:

```
let bag_client = bag::BagClient::new("Your user agent", "Your BAG key");
let buildings = bag.get_panden("0268010000084126").await?;
...
```

For finding lot information using a lot code, `brk`:

```
let brk_client = brk::BrkClient::new("Your user agent", brk::CoordinateSpace::Gps);
let lot = brk_client.get_lot("HTT02", "M", "5038").await?;
...
```

## Test upstreams

Test if upstreams produce expected output:

```
BAG_API_KEY=<your key> cargo test
```