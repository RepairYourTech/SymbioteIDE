for (var i = 0; i < workspace.clientList().length; i++) {
    var c = workspace.clientList()[i];
    if (c.caption === "Symbiote") {
        workspace.activeClient = c;
    }
}
