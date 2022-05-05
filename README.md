# Geo APIs for Pect

## Usage

Exposed functions expect a client object for calling upstreams.

For `locatieserver`:

```
let client = reqwest::Client::new();
let suggestions = locatieserver::suggest_concrete("6512EX", "26").await?;
...
```

For `bag`:

```
let bag_client = BagClient::new("Your BAG key", "Your user agent", Duration::new(5, 0));
let buildings = bag::get_panden(&bag_client, "0268010000084126").await?;
...
```

For `brk`:

```
let brk_client = BrkClient::new("Your user agent");
let lot = brk::get_lot(&brk_client, "HTT02", "M", "5038").await?;
...
```

## Test upstream

Test if upstreams produce expected output:

```
BAG_API_KEY=<your key> cargo test
```