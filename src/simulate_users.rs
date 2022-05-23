use std::{thread, time};
use log::*;
use url::Url;

use tungstenite::{connect, Error, Message, Result};


fn send_line(line: &str) -> Result<()> {
    let case_url = Url::parse("ws://localhost:3030/game").unwrap();
    let (mut socket, _) = connect(case_url)?;

    socket.write_message(Message::Text(line.into())).unwrap();
    Ok(())
}

fn main() {

    // Data is of a konva line.
    // The points field is flattened is x, y consecutive pairs
    let line = r#"
    {
        "attrs":
        {
            "stroke": "/#df4b26",
            "strokeWidth": 5,
            "lineCap": "round",
            "points":
            [
                164.00105119637686,
                856.3408239439925,
                164.00105119637686,
                856.3408239439925,
                170.00108965478088,
                856.3408239439925,
                177.00113452291893,
                855.3408151952004,
                190.001217849461,
                855.3408151952004,
                198.00126912733302,
                855.3408151952004,
                210.0013460441411,
                855.3408151952004,
                223.00142937068316,
                855.3408151952004,
                237.00151910695922,
                855.3408151952004,
                250.0016024335013,
                855.3408151952004,
                263.00168576004336,
                855.3408151952004,
                284.0018203644575,
                855.3408151952004,
                295.00189087153154,
                855.3408151952004,
                311.00199342727564,
                855.3408151952004,
                333.0021344414237,
                855.3408151952004,
                352.0022562263698,
                854.3408064464085,
                366.0023459626459,
                854.3408064464085,
                396.00253825466604,
                853.3407976976165,
                417.0026728590802,
                851.3407802000324,
                447.0028651511003,
                849.3407627024484,
                475.00304462365244,
                849.3407627024484,
                493.00315999886453,
                849.3407627024484,
                530.0033971590227,
                848.3407539536563,
                546.0034997147668,
                848.3407539536563,
                564.0036150899789,
                848.3407539536563,
                591.003788152797,
                848.3407539536563,
                602.0038586598711,
                848.3407539536563,
                618.0039612156152,
                848.3407539536563,
                636.0040765908273,
                848.3407539536563,
                651.0041727368374,
                848.3407539536563,
                663.0042496536454,
                848.3407539536563,
                681.0043650288575,
                849.3407627024484,
                698.0044739943356,
                850.3407714512404,
                707.0045316819417,
                851.3407802000324,
                723.0046342376858,
                851.3407802000324,
                741.0047496128979,
                853.3407976976165,
                753.004826529706,
                856.3408239439925,
                762.004884217312,
                857.3408326927845,
                777.004980363322,
                858.3408414415766,
                791.0050700995981,
                859.3408501903685,
                800.0051277872042,
                861.3408676879526,
                808.0051790650762,
                861.3408676879526,
                821.0052623916182,
                861.3408676879526,
                835.0053521278943,
                863.3408851855366,
                847.0054290447024,
                863.3408851855366,
                855.0054803225744,
                863.3408851855366,
                871.0055828783185,
                863.3408851855366,
                881.0056469756586,
                863.3408851855366,
                893.0057238924667,
                861.3408676879526,
                902.0057815800727,
                861.3408676879526,
                914.0058584968807,
                860.3408589391606,
                928.0059482331568,
                860.3408589391606,
                937.0060059207628,
                860.3408589391606,
                945.006057198635,
                860.3408589391606,
                953.0061084765069,
                860.3408589391606,
                963.006172573847,
                863.3408851855366,
                972.0062302614531,
                864.3408939343287,
                981.0062879490591,
                867.3409201807048,
                986.0063199977291,
                869.3409376782888,
                992.0063584561332,
                870.3409464270809,
                996.0063840950692,
                870.3409464270809,
                1001.0064161437392,
                871.3409551758729,
                1007.0064546021432,
                874.3409814222489,
                1014.0064994702813,
                876.3409989198329,
                1020.0065379286852,
                876.3409989198329,
                1026.0065763870894,
                878.341016417417,
                1031.0066084357593,
                878.341016417417,
                1034.0066276649613,
                879.3410251662091,
                1038.0066533038973,
                880.341033915001,
                1043.0066853525675,
                881.3410426637931,
                1047.0067109915035,
                881.3410426637931,
                1053.0067494499074,
                881.3410426637931,
                1057.0067750888436,
                883.3410601613772,
                1062.0068071375135,
                883.3410601613772,
                1067.0068391861835,
                883.3410601613772,
                1071.0068648251195,
                883.3410601613772,
                1074.0068840543215,
                884.3410689101692,
                1075.0068904640557,
                884.3410689101692,
                1077.0069032835236,
                884.3410689101692,
                1080.0069225127256,
                884.3410689101692,
                1082.0069353321935,
                884.3410689101692,
                1083.0069417419277,
                884.3410689101692,
                1083.0069417419277,
                884.3410689101692,
                1084.0069481516616,
                884.3410689101692,
                1085.0069545613956,
                884.3410689101692,
                1086.0069609711297,
                884.3410689101692,
                1087.0069673808637,
                884.3410689101692
            ]
        },
        "className": "Line"
    }
    "#;
    let ten_millis = time::Duration::from_millis(1000);
    let total = 50;
    for n in 1..total {
        println!("Send line {} out of {}", n, total);
        send_line(line);
        thread::sleep(ten_millis);
    }



}
