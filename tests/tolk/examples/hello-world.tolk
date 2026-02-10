const OP_TRANSFER = 0xf8a7ea5;
const OP_BURN = 0x595f07bc;
const MIN_BALANCE = 10000000;
const MAX_SUPPLY = 1000000000;

struct Storage {
    totalSupply: coins,
    adminAddress: address,
    paused: bool
}

fun Storage.load() {
    return Storage.fromCell(contract.getData());
}

fun Storage.save(self) {
    contract.setData(self.toCell());
}

struct(0x7e8764ef) MintTokens {
    amount: coins,
    recipient: address
}

struct(0x595f07bc) BurnNotification {
    amount: coins,
    senderAddress: address
}

struct(0x3a752f06) Pause {}
struct(0x4b6f7089) Unpause {}
struct(0xd53276db) ChangeAdmin {
    newAdmin: address
}

type AllowedMessage =
    | MintTokens
    | BurnNotification
    | Pause
    | Unpause
    | ChangeAdmin;

fun requireAdmin(senderAddress: address, adminAddress: address) {
    if (senderAddress != adminAddress) {
        throw 401;
    }
}

fun onInternalMessage(in: InMessage) {
    val msg = lazy AllowedMessage.fromSlice(in.body);
    var storage = lazy Storage.load();

    match (msg) {
        MintTokens => {
            requireAdmin(in.senderAddress, storage.adminAddress);
            assert(storage.paused == false) throw 403;
            val newSupply: coins = storage.totalSupply + msg.amount;
            assert(newSupply <= MAX_SUPPLY) throw 404;
            storage.totalSupply = newSupply;
            storage.save();
        }

        BurnNotification => {
            assert(msg.amount > 0) throw 400;
            if (storage.totalSupply >= msg.amount) {
                storage.totalSupply -= msg.amount;
            }
            storage.save();
        }

        Pause => {
            requireAdmin(in.senderAddress, storage.adminAddress);
            storage.paused = true;
            storage.save();
        }

        Unpause => {
            requireAdmin(in.senderAddress, storage.adminAddress);
            storage.paused = false;
            storage.save();
        }

        ChangeAdmin => {
            requireAdmin(in.senderAddress, storage.adminAddress);
            storage.adminAddress = msg.newAdmin;
            storage.save();
        }

        else => {
            assert(in.body.isEmpty()) throw 0xFFFF;
        }
    }
}

fun computeFee(amount: coins, basisPoints: int): coins {
    val fee: coins = amount * basisPoints / 10000;
    if (fee < 1) {
        return 1;
    }
    return fee;
}

fun pow(base: int, exp: int): int {
    var result: int = 1;
    var i: int = 0;
    while (i < exp) {
        result *= base;
        i += 1;
    }
    return result;
}

get fun totalSupply(): coins {
    val storage = lazy Storage.load();
    return storage.totalSupply;
}

get fun isPaused(): bool {
    val storage = lazy Storage.load();
    return storage.paused;
}

get fun adminAddress(): address {
    val storage = lazy Storage.load();
    return storage.adminAddress;
}
