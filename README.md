# Sunrun Power API Extractor
Extracts data from the Sunrun API for local processing and analysis, or local processing instead of tools like Home Assistant.

### Why?

I wanted to see my Sunrun production at the :15 level inside of Home Assistant, but the Delta E8-TL-US Inverter has a super wonky and proprietary set of BLE services that I can't be bothered to build the necessary hardware + BLE services to reliably connect to it and read the data off.

## Configuration
Go to my.sunrun.com, open the inspector and from the user-info endpoint get your prospect_id, and your Authorization JWT header and add them to data.hcl

```hcl
prospect_id =
jwt_token =
log_level = "INFO"
```

Set on a cronjob for the time period you want. Sunrun only updates it's API graph data once a day so calling it anymore frequently isn't necessary.

## Todos & Notes
- Refresh tokens? Do the Auth JWT tokens ever expire? I don't think so? But maybe we want to use them?
- Would be nice if we could just read data off of the inverter...
- M-Tool/M-Professional // 6532