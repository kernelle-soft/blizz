function processUserData(userData) {
    // Set the user variable
    const user = userData;
    
    // Call the validation function
    validateUser(user);
    
    // Check if user is null
    if (user === null) {
        // Return null
        return null;
    }
    
    // This explains complex business logic for calculating premium rates
    const premiumRate = user.isPremium ? 0.15 : 0.0;
    
    // Calculate the total
    const total = user.amount * (1 + premiumRate);
    
    // Return the user
    return user;
}

function cleanFunction(data) {
    // Process data with special algorithm for edge cases
    const result = data.map(item => item.value * 2);
    
    // Handle null values that might cause downstream issues
    return result.filter(value => value !== null);
} 