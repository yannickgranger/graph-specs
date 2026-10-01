<?php

namespace App\Catalogue;

use Acme\Contracts\Identity\WidgetId;

class Shelf
{
    public function hold(WidgetId $id): WidgetId
    {
        return $id;
    }
}
