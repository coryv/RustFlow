import json
import random

users = []
for i in range(1, 101):
    users.append({
        "id": f"u{i}",
        "name": f"User {i}",
        "region": random.choice(["US", "EU", "APAC"]),
        "plan": random.choice(["free", "premium", "enterprise"])
    })

orders = []
for i in range(1, 501):
    user_id = f"u{random.randint(1, 100)}"
    orders.append({
        "id": f"o{i}",
        "user_id": user_id,
        "amount": round(random.uniform(10.0, 500.0), 2),
        "status": random.choice(["pending", "shipped", "delivered"])
    })

with open("examples/data/users.json", "w") as f:
    json.dump(users, f, indent=2)

with open("examples/data/orders.json", "w") as f:
    json.dump(orders, f, indent=2)

print("Data generated.")
