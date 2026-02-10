const MIN_BALANCE = 10000000;

struct Storage {
    counter: uint64
}

fun Storage.load() {
    return Storage.fromCell(contract.getData());
}

fun Storage.save(self) {
    contract.setData(self.toCell());
}

struct(0x7e8764ef) IncreaseCounter {
    increaseBy: uint32
}

fun onInternalMessage(in: InMessage) {
    var storage = lazy Storage.load();
    val msg = IncreaseCounter.fromSlice(in.body);
    storage.counter += msg.increaseBy;
    storage.save();
}

get fun currentCounter(): int {
    val storage = lazy Storage.load();
    return storage.counter;
}
