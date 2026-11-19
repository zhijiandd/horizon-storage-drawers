fn menu_actor(actor: u128, game: &Game<'_>) -> u128 {
    // The integrated server keeps the save owner's UUID even when it has no player body.
    // During a menu transaction, let the item writer and its rollback resolve the online guest.
    if game.player.is_none()
        && game.lan_acting_uuid.is_none()
        && storagedrawers::menu_click_player() == Some(actor)
    {
        0
    } else {
        actor
    }
}
