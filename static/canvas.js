window.addEventListener("load", () => {

    const canvas = document.querySelector("#canvas");
    const ctx = canvas.getContext("2d");


    canvas.height = window.innerHeight;
    canvas.width = window.innerWidth;
    const boundings = canvas.getBoundingClientRect();

    // flag used to prevent lines connecting from different users
    let isDataFromUser = true;

    let painting = false;

    // ws
    const uri = 'ws://' + location.host + '/chat';
    const ws = new WebSocket(uri);
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

    // canvas event listeners
    canvas.addEventListener("mousedown", (e)=>{
        painting=true;
        handleDraw(e);

    });
    canvas.addEventListener("mouseup", ()=>{
        painting=false;
        ctx.beginPath();

    });
    canvas.addEventListener("mousemove", handleDraw);

    function handleDraw(e){
        if(!painting) return;
        if (!isDataFromUser){
            ctx.beginPath();
            isDataFromUser = true;
        }
        
        let mouseX = Math.round(e.clientX - boundings.left);
        let mouseY = Math.round(e.clientY - boundings.top);

        console.log(`transmitting data: {"x": ${mouseX}, "y":${mouseY}}`);

        // send data
        ws.send(`{"x": ${mouseX}, "y":${mouseY}}`);

        draw(mouseX, mouseY);


    }

    function draw(x, y){
        ctx.lineWidth = 10;
        ctx.lineCap = "round";
        ctx.lineTo(x, y);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(x, y);
    }




});