function textToBinary(string) {
    return string.split('').map( char => char.charCodeAt(0).toString(2)).join(' ');
}

function binaryToText(string){
    return string.split(' ').map(char => String.fromCharCode(parseInt(char, 2))).join('');
}
