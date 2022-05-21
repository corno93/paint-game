window.addEventListener("load", () => {

    const uri = 'ws://' + location.host + '/game';
    const ws = new WebSocket(uri);

    var stage = new Konva.Stage({
      container: 'container',   // id of container <div>
      width: window.innerWidth,
      height: window.innerHeight
    });

    var layer = new Konva.Layer();
    stage.add(layer);

    let isDrawing = false;
    let lastLine = null;

    stage.on("mousedown touchstart", mousedownHandler);
    stage.on("mouseup touchend", mouseupHandler);
    stage.on("mousemove", mousemoveHandler);

    function mousedownHandler() {
        isDrawing = true;
        var pos = stage.getPointerPosition();
        lastLine = new Konva.Line({
            stroke: '#df4b26',
            strokeWidth: 5,
            globalCompositeOperation: 'source-over',
            // round cap for smoother lines
            lineCap: 'round',
            // add point twice, so we have some drawings even on a simple click
            points: [pos.x, pos.y, pos.x, pos.y],
        });
        layer.add(lastLine);
    }

    function mouseupHandler(){
        isDrawing = false;
        console.log("line: ", lastLine.toJSON());
        ws.send(`${lastLine.toJSON()}`);
    }

    function mousemoveHandler(e) {
        if (!isDrawing) {
            return;
        }
        // prevent scrolling on touch devices
        e.evt.preventDefault();
        const pos = stage.getPointerPosition();
        var newPoints = lastLine.points().concat([pos.x, pos.y]);
        lastLine.points(newPoints);
    }


    ws.onopen = function() {
        console.log("connected");
    };
    ws.onmessage = function(msg) {

        if (isDataFromUser){
            ctx.beginPath();
            isDataFromUser = false;
        }

        let mouseEvent = JSON.parse(msg.data);
        console.log(`recieved data: {"x": ${mouseEvent.x}, "y":${mouseEvent.y}}`);

        draw(mouseEvent.x, mouseEvent.y)


    };
    ws.onclose = function() {
        console.log("disconnected");
    };


});
