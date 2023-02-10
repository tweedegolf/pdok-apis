# PDOK APIs

Provides partial support for consuming APIs of PDOK. These APIs provide information on addresses, buildings and lots in the Netherlands.

## Usage
Clients are created using a builder. You can change optional settings by using the chaining methods. New Client builders can be created by implementing the `ClientBuilder` trait

For finding an address information using a postal code and housenumber, `locatieserver`:

``` rust
let lookup_client = lookup::LookupClientBuilder::new("Your user agent");
let suggestions = lookup_client.suggest_concrete("6512EX", "26").await?;
...
```

For getting building information, `bag`:

``` rust
let bag_client = BagClientBuilder::new(user_agent, api_key)
        .accept_crs(BagCoordinateSpace::Rijksdriehoek)
        .build()

let buildings = bag.get_panden("0268010000084126").await?;
...
```

For finding lot information using a lot code, `brk`:

``` rust
let brk_client = BrkClientBuilder::new(APP_USER_AGENT)
        .connection_timeout_secs(20)
        .request_timeout_secs(60)
        .accept_crs(BagCoordinateSpace::Rijksdriehoek)
        .build();
let lot = brk_client.get_lot("HTT02", "M", "5038").await?;
...
```

## Test upstreams

Test if upstreams produce expected output:

```
BAG_API_KEY=<your key> cargo test
```
